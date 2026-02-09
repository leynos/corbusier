//! Cluster lifecycle helpers for `PostgreSQL` integration tests.
mod env_utils;
mod fs_utils;
mod worker_helpers;
use self::env_utils::{drop_privileges_if_root, env_vars_to_os, worker_env_changes};
use self::fs_utils::{cleanup_stale_postmaster_pid, sync_password_from_file, sync_port_from_pid};
use crate::test_helpers::EnvVarGuard;
use diesel::prelude::*;
use once_cell::sync::OnceCell;
use pg_embedded_setup_unpriv::worker_process_test_api::{
    WorkerOperation, WorkerRequest, WorkerRequestArgs, run as run_worker,
};
use pg_embedded_setup_unpriv::{ExecutionPrivileges, TestBootstrapSettings, bootstrap_for_tests};
use postgresql_embedded::{PostgreSQL, Settings, Status};
use rstest::fixture;
use std::io::{self, Write};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::runtime::Runtime;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
static SHARED_CLUSTER: OnceCell<ManagedCluster> = OnceCell::new();
static TEMPLATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
/// RAII guard for temporary test databases.
///
/// Automatically drops the database when the guard goes out of scope.
pub struct TemporaryDatabase {
    cluster: &'static ManagedCluster,
    name: String,
    url: String,
}
impl TemporaryDatabase {
    const fn new(cluster: &'static ManagedCluster, name: String, url: String) -> Self {
        Self { cluster, name, url }
    }
    /// Returns the database name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Returns the database URL.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}
impl Drop for TemporaryDatabase {
    fn drop(&mut self) {
        if let Err(err) = self.cluster.drop_database(&self.name)
            && writeln!(
                io::stderr(),
                "Failed to drop test database {}: {err}",
                self.name
            )
            .is_err()
        {
            // Ignore stderr failures during cleanup reporting.
        }
    }
}
/// Shared `PostgreSQL` cluster handle for integration tests.
pub type PostgresCluster = &'static ManagedCluster;
/// Lightweight connection wrapper for building database URLs.
#[derive(Debug, Clone)]
pub struct ClusterConnection {
    settings: Settings,
}
impl ClusterConnection {
    /// Builds a database URL for the provided database name.
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
async fn setup_postgres(postgres: &mut PostgreSQL) -> Result<(), BoxError> {
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
    Ok(())
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
    /// Returns a connection helper for generating database URLs.
    #[must_use]
    pub fn connection(&self) -> ClusterConnection {
        ClusterConnection {
            settings: self.bootstrap.settings.clone(),
        }
    }
    /// Creates a database using the provided template name.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created.
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

    /// Drops the named database from the cluster.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be dropped.
    pub fn drop_database(&self, db_name: &str) -> Result<(), BoxError> {
        let sql = format!("DROP DATABASE {}", quote_identifier(db_name));
        self.execute_admin_sql(&sql)
    }

    /// Ensures a template database exists and has migrations applied.
    ///
    /// # Errors
    ///
    /// Returns an error if template creation or migration fails.
    pub async fn ensure_template_exists<F>(
        &self,
        template: &str,
        migrate: F,
    ) -> Result<(), BoxError>
    where
        F: FnOnce(String) -> Result<(), BoxError> + Send + 'static,
    {
        let admin_url = self.connection().database_url("postgres");
        let template_name = template.to_owned();
        let template_name_for_drop = template.to_owned();

        tokio::task::spawn_blocking(move || {
            let lock = TEMPLATE_LOCK.get_or_init(|| Mutex::new(()));
            let _guard = lock
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            if database_exists_with_url(&admin_url, &template_name)? {
                return Ok(());
            }

            create_database_with_url(&admin_url, &template_name)?;
            if let Err(err) = migrate(template_name) {
                drop_template_after_failure(&admin_url, &template_name_for_drop);
                return Err(err);
            }
            Ok(())
        })
        .await
        .map_err(|err| Box::new(err) as BoxError)?
    }

    /// Creates a temporary database from a template for test usage.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created from the template.
    #[expect(clippy::unused_async, reason = "Part of async API for consistency")]
    pub async fn temporary_database_from_template(
        &'static self,
        db_name: &str,
        template: &str,
    ) -> Result<TemporaryDatabase, BoxError> {
        self.create_database_from_template(db_name, template)?;
        let database_url = self.connection().database_url(db_name);
        Ok(TemporaryDatabase::new(
            self,
            db_name.to_owned(),
            database_url,
        ))
    }

    fn start(&mut self) -> Result<(), BoxError> {
        let env_vars = env_vars_to_os(&self.env_vars);
        cleanup_stale_postmaster_pid(&self.bootstrap.settings)?;
        match self.bootstrap.privileges {
            ExecutionPrivileges::Root => {
                let started = self.start_via_worker().is_ok()
                    && database_exists_with_url(
                        &self.connection().database_url("postgres"),
                        "postgres",
                    )
                    .is_ok();
                if started {
                    Ok(())
                } else {
                    let env_guard = drop_privileges_if_root("nobody", &env_vars)?;
                    self.start_in_process(&env_vars, env_guard)
                }
            }
            ExecutionPrivileges::Unprivileged => self.start_in_process(&env_vars, None),
        }
    }

    fn start_in_process(
        &mut self,
        env_vars: &[(std::ffi::OsString, Option<std::ffi::OsString>)],
        env_guard: Option<EnvVarGuard>,
    ) -> Result<(), BoxError> {
        let settings = self.bootstrap.settings.clone();
        let _env_guard = env_guard.unwrap_or_else(|| EnvVarGuard::set_many(env_vars));

        // Run PostgreSQL startup in a separate thread to avoid runtime nesting issues
        let result = std::thread::scope(|s| {
            s.spawn(|| {
                let mut postgres = PostgreSQL::new(settings);

                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| Box::new(e) as BoxError)?;

                runtime.block_on(setup_postgres(&mut postgres))?;

                let cluster_settings = postgres.settings().clone();
                Ok::<(Runtime, PostgreSQL, Settings), BoxError>((
                    runtime,
                    postgres,
                    cluster_settings,
                ))
            })
            .join()
            .map_err(|_| Box::new(std::io::Error::other("thread panicked")) as BoxError)?
        });

        let (runtime, postgres, cluster_settings) = result?;
        self.bootstrap.settings = cluster_settings;
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

    fn execute_admin_sql(&self, sql: &str) -> Result<(), BoxError> {
        execute_admin_sql_with_url(&self.connection().database_url("postgres"), sql)
    }
}

fn drop_template_after_failure(admin_url: &str, template_name: &str) {
    let drop_err = match drop_database_with_url(admin_url, template_name) {
        Ok(()) => return,
        Err(err) => err,
    };

    if writeln!(
        io::stderr(),
        "Failed to drop template database {template_name} after migration error: {drop_err}"
    )
    .is_err()
    {
        // Ignore stderr failures during cleanup reporting.
    }
}

fn execute_admin_sql_with_url(admin_url: &str, sql: &str) -> Result<(), BoxError> {
    let mut conn = PgConnection::establish(admin_url).map_err(|err| Box::new(err) as BoxError)?;
    diesel::sql_query(sql)
        .execute(&mut conn)
        .map_err(|err| Box::new(err) as BoxError)?;
    Ok(())
}

fn create_database_with_url(admin_url: &str, db_name: &str) -> Result<(), BoxError> {
    let sql = format!("CREATE DATABASE {}", quote_identifier(db_name));
    execute_admin_sql_with_url(admin_url, &sql)
}

fn drop_database_with_url(admin_url: &str, db_name: &str) -> Result<(), BoxError> {
    let sql = format!("DROP DATABASE {}", quote_identifier(db_name));
    execute_admin_sql_with_url(admin_url, &sql)
}

fn database_exists_with_url(admin_url: &str, db_name: &str) -> Result<bool, BoxError> {
    #[derive(diesel::QueryableByName)]
    struct ExistsRow {
        #[diesel(sql_type = diesel::sql_types::Bool)]
        exists: bool,
    }

    let mut conn = PgConnection::establish(admin_url).map_err(|err| Box::new(err) as BoxError)?;
    let row =
        diesel::sql_query("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1) AS exists")
            .bind::<diesel::sql_types::Text, _>(db_name)
            .get_result::<ExistsRow>(&mut conn)
            .map_err(|err| Box::new(err) as BoxError)?;
    Ok(row.exists)
}

impl Drop for ManagedCluster {
    fn drop(&mut self) {
        if let Err(err) = self.stop()
            && writeln!(io::stderr(), "Failed to stop PostgreSQL cluster: {err}").is_err()
        {
            // Ignore stderr failures during cleanup reporting.
        }
    }
}

/// Provides a `PostgreSQL` test cluster suitable for the current test runner.
#[fixture]
pub fn postgres_cluster() -> Result<PostgresCluster, BoxError> {
    shared_cluster()
}

fn shared_cluster() -> Result<PostgresCluster, BoxError> {
    SHARED_CLUSTER
        .get_or_try_init(ManagedCluster::new)
        .map_err(|err| {
            Box::new(std::io::Error::other(format!(
                "SKIP-TEST-CLUSTER: failed to start PostgreSQL: {err}"
            ))) as BoxError
        })
}

fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
