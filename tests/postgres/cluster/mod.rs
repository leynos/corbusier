//! Cluster lifecycle helpers for `PostgreSQL` integration tests.

mod env_utils;
mod fs_utils;
mod worker_helpers;

use self::env_utils::{env_vars_to_os, worker_env_changes};
use self::fs_utils::{sync_password_from_file, sync_port_from_pid};
use super::helpers::test_runtime;
use crate::test_helpers::EnvVarGuard;
use diesel::prelude::*;
use pg_embedded_setup_unpriv::worker_process_test_api::{
    WorkerOperation, WorkerRequest, WorkerRequestArgs, run as run_worker,
};
use pg_embedded_setup_unpriv::{ExecutionPrivileges, TestBootstrapSettings, bootstrap_for_tests};
use postgresql_embedded::{PostgreSQL, Settings, Status};
use rstest::fixture;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::runtime::Runtime;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

static SHARED_CLUSTER: OnceLock<ManagedCluster> = OnceLock::new();
static TEMPLATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Shared `PostgreSQL` cluster handle for integration tests.
pub type PostgresCluster = &'static ManagedCluster;

/// Lightweight connection wrapper for building database URLs.
#[derive(Debug, Clone)]
pub struct ClusterConnection {
    settings: Settings,
}

impl ClusterConnection {
    #[must_use]
    pub fn database_url(&self, database: &str) -> String {
        self.settings.url(database)
    }
}

/// Managed embedded `PostgreSQL` cluster for test lifecycles.
pub struct ManagedCluster {
    bootstrap: TestBootstrapSettings,
    env_vars: Vec<(String, Option<String>)>,
    runtime: Option<Runtime>,
    postgres: Option<PostgreSQL>,
}

impl ManagedCluster {
    fn new() -> Result<Self, BoxError> {
        let (worker_env, port_guard) = worker_env_changes()?;
        let worker_guard = EnvVarGuard::set_many(&worker_env);
        let mut bootstrap = bootstrap_for_tests().map_err(|err| Box::new(err) as BoxError)?;
        drop(worker_guard);
        drop(port_guard);
        sync_password_from_file(&mut bootstrap.settings)?;
        let env_vars = bootstrap.environment.to_env();
        let mut cluster = Self {
            bootstrap,
            env_vars,
            runtime: None,
            postgres: None,
        };
        cluster.start()?;
        Ok(cluster)
    }

    #[must_use]
    pub fn connection(&self) -> ClusterConnection {
        ClusterConnection {
            settings: self.bootstrap.settings.clone(),
        }
    }

    pub fn create_database_from_template(
        &self,
        db_name: &str,
        template: &str,
    ) -> Result<(), BoxError> {
        let sql = format!(
            "CREATE DATABASE {} TEMPLATE {}",
            quote_identifier(db_name),
            quote_identifier(template),
        );
        self.execute_admin_sql(&sql)
    }

    pub fn drop_database(&self, db_name: &str) -> Result<(), BoxError> {
        let sql = format!("DROP DATABASE {}", quote_identifier(db_name));
        self.execute_admin_sql(&sql)
    }

    pub fn ensure_template_exists<F>(&self, template: &str, migrate: F) -> Result<(), BoxError>
    where
        F: FnOnce(&str) -> Result<(), BoxError>,
    {
        let lock = TEMPLATE_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if self.database_exists(template)? {
            return Ok(());
        }

        self.create_database(template)?;
        if let Err(err) = migrate(template) {
            self.drop_database(template)?;
            return Err(err);
        }
        Ok(())
    }

    fn start(&mut self) -> Result<(), BoxError> {
        match self.bootstrap.privileges {
            ExecutionPrivileges::Root => self.start_via_worker(),
            ExecutionPrivileges::Unprivileged => self.start_in_process(),
        }
    }

    fn start_in_process(&mut self) -> Result<(), BoxError> {
        let runtime = test_runtime()?;
        let env_guard = EnvVarGuard::set_many(&env_vars_to_os(&self.env_vars));
        let mut postgres = PostgreSQL::new(self.bootstrap.settings.clone());
        runtime.block_on(async {
            postgres
                .setup()
                .await
                .map_err(|err| Box::new(err) as BoxError)?;
            if !matches!(postgres.status(), Status::Started) {
                postgres
                    .start()
                    .await
                    .map_err(|err| Box::new(err) as BoxError)?;
            }
            Ok::<(), BoxError>(())
        })?;
        drop(env_guard);
        self.bootstrap.settings = postgres.settings().clone();
        sync_port_from_pid(&mut self.bootstrap.settings)?;
        self.runtime = Some(runtime);
        self.postgres = Some(postgres);
        Ok(())
    }

    fn start_via_worker(&mut self) -> Result<(), BoxError> {
        self.run_worker_operation(WorkerOperation::Setup, self.bootstrap.setup_timeout)?;
        self.run_worker_operation(WorkerOperation::Start, self.bootstrap.start_timeout)?;
        sync_port_from_pid(&mut self.bootstrap.settings)?;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), BoxError> {
        let Some(postgres) = self.postgres.take() else {
            if matches!(self.bootstrap.privileges, ExecutionPrivileges::Root) {
                self.run_worker_operation(WorkerOperation::Stop, self.bootstrap.shutdown_timeout)?;
            }
            return Ok(());
        };

        let Some(runtime) = &self.runtime else {
            return Ok(());
        };

        runtime.block_on(async {
            postgres
                .stop()
                .await
                .map_err(|err| Box::new(err) as BoxError)
        })?;
        Ok(())
    }

    fn run_worker_operation(
        &self,
        operation: WorkerOperation,
        timeout: Duration,
    ) -> Result<(), BoxError> {
        let worker = self.bootstrap.worker_binary.as_ref().ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "PG_EMBEDDED_WORKER is not set for worker operation",
            )) as BoxError
        })?;
        let args = WorkerRequestArgs {
            worker: worker.as_path(),
            settings: &self.bootstrap.settings,
            env_vars: &self.env_vars,
            operation,
            timeout,
        };
        run_worker(&WorkerRequest::new(args)).map_err(|err| Box::new(err) as BoxError)?;
        Ok(())
    }

    fn admin_connection(&self) -> Result<PgConnection, BoxError> {
        let url = self.connection().database_url("postgres");
        PgConnection::establish(&url).map_err(|err| Box::new(err) as BoxError)
    }

    fn execute_admin_sql(&self, sql: &str) -> Result<(), BoxError> {
        let mut conn = self.admin_connection()?;
        diesel::sql_query(sql)
            .execute(&mut conn)
            .map_err(|err| Box::new(err) as BoxError)?;
        Ok(())
    }

    fn create_database(&self, db_name: &str) -> Result<(), BoxError> {
        let sql = format!("CREATE DATABASE {}", quote_identifier(db_name));
        self.execute_admin_sql(&sql)
    }

    fn database_exists(&self, db_name: &str) -> Result<bool, BoxError> {
        #[derive(diesel::QueryableByName)]
        struct ExistsRow {
            #[diesel(sql_type = diesel::sql_types::Bool)]
            exists: bool,
        }

        let mut conn = self.admin_connection()?;
        let row = diesel::sql_query(
            "SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1) AS exists",
        )
        .bind::<diesel::sql_types::Text, _>(db_name)
        .get_result::<ExistsRow>(&mut conn)
        .map_err(|err| Box::new(err) as BoxError)?;
        Ok(row.exists)
    }
}

impl Drop for ManagedCluster {
    fn drop(&mut self) {
        drop(self.stop());
    }
}

/// Provides a `PostgreSQL` test cluster suitable for the current test runner.
#[fixture]
pub fn postgres_cluster() -> PostgresCluster {
    shared_cluster()
}

fn shared_cluster() -> PostgresCluster {
    SHARED_CLUSTER.get_or_init(|| match ManagedCluster::new() {
        Ok(cluster) => cluster,
        Err(err) => panic!("SKIP-TEST-CLUSTER: failed to start PostgreSQL: {err}"),
    })
}

fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
