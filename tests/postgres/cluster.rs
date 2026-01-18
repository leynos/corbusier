//! Cluster lifecycle helpers for `PostgreSQL` integration tests.

use super::helpers::test_runtime;
use crate::test_helpers::EnvVarGuard;
use cap_std::ambient_authority;
use cap_std::fs::{Dir, Permissions, PermissionsExt};
use diesel::prelude::*;
use pg_embedded_setup_unpriv::worker_process_test_api::{
    WorkerOperation, WorkerRequest, WorkerRequestArgs, run as run_worker,
};
use pg_embedded_setup_unpriv::{
    ExecutionPrivileges, TestBootstrapSettings, bootstrap_for_tests, detect_execution_privileges,
};
use postgresql_embedded::{PostgreSQL, Settings, Status};
use rstest::fixture;
use std::ffi::{OsStr, OsString};
use std::net::TcpListener;
use std::path::Path;
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
        let worker_env = worker_env_changes()?;
        let worker_guard = EnvVarGuard::set_many(&worker_env);
        let mut bootstrap = bootstrap_for_tests().map_err(|err| Box::new(err) as BoxError)?;
        drop(worker_guard);
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

fn env_vars_to_os(env_vars: &[(String, Option<String>)]) -> Vec<(OsString, Option<OsString>)> {
    env_vars
        .iter()
        .map(|(key, value)| (OsString::from(key), value.as_ref().map(OsString::from)))
        .collect()
}

fn sync_password_from_file(settings: &mut Settings) -> Result<(), BoxError> {
    let (dir, file_name) = open_parent_dir(&settings.password_file)?;
    match dir.read_to_string(file_name) {
        Ok(contents) => {
            let password = contents.trim_end();
            if !password.is_empty() {
                password.clone_into(&mut settings.password);
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(Box::new(err) as BoxError),
    }
    Ok(())
}

fn sync_port_from_pid(settings: &mut Settings) -> Result<(), BoxError> {
    let data_dir = open_ambient_dir(&settings.data_dir)?;
    let contents = match data_dir.read_to_string("postmaster.pid") {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Box::new(err) as BoxError),
    };

    let port_line = contents.lines().nth(3).map(str::trim);
    let Some(port_value) = port_line else {
        return Ok(());
    };
    let Ok(port) = port_value.parse::<u16>() else {
        return Ok(());
    };
    settings.port = port;
    Ok(())
}

fn worker_env_changes() -> Result<Vec<(OsString, Option<OsString>)>, BoxError> {
    let port_override = resolve_pg_port()?;

    let mut changes = Vec::new();
    if let Some(port) = port_override {
        changes.push((OsString::from("PG_PORT"), Some(port)));
    }

    if matches!(detect_execution_privileges(), ExecutionPrivileges::Root)
        && std::env::var_os("PG_EMBEDDED_WORKER").is_none()
    {
        let worker_path = locate_pg_worker_path().ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "PG_EMBEDDED_WORKER is not set and pg_worker binary was not found",
            )) as BoxError
        })?;

        let prepared_worker = prepare_pg_worker(&worker_path)?;
        changes.push((OsString::from("PG_EMBEDDED_WORKER"), Some(prepared_worker)));
    }

    Ok(changes)
}

fn locate_pg_worker_path() -> Option<OsString> {
    std::env::var_os("CARGO_BIN_EXE_pg_worker")
        .or_else(locate_pg_worker_near_target)
        .or_else(locate_pg_worker_in_path)
        .or_else(locate_pg_worker_from_env)
}

fn prepare_pg_worker(worker: &OsString) -> Result<OsString, BoxError> {
    static WORKER_CACHE: OnceLock<OsString> = OnceLock::new();
    if let Some(cached) = WORKER_CACHE.get() {
        return Ok(cached.clone());
    }

    let source = std::path::PathBuf::from(worker);
    let destination_path =
        std::env::temp_dir().join(format!("pg_worker_{pid}", pid = std::process::id()));
    let (source_dir, source_name) = open_parent_dir(&source)?;
    let (destination_dir, destination_name) = open_parent_dir(&destination_path)?;

    if destination_dir.exists(destination_name) {
        destination_dir
            .remove_file(destination_name)
            .map_err(|err| Box::new(err) as BoxError)?;
    }

    source_dir
        .copy(source_name, &destination_dir, destination_name)
        .map_err(|err| Box::new(err) as BoxError)?;

    #[cfg(unix)]
    {
        destination_dir
            .set_permissions(destination_name, Permissions::from_mode(0o755))
            .map_err(|err| Box::new(err) as BoxError)?;
    }

    let destination = destination_path.into_os_string();
    if WORKER_CACHE.set(destination.clone()).is_err() {
        // Another test stored the prepared worker path first.
    }
    Ok(destination)
}

fn open_ambient_dir(path: &Path) -> Result<Dir, BoxError> {
    Dir::open_ambient_dir(path, ambient_authority()).map_err(|err| Box::new(err) as BoxError)
}

fn open_parent_dir(path: &Path) -> Result<(Dir, &OsStr), BoxError> {
    let file_name = path.file_name().ok_or_else(|| {
        Box::new(std::io::Error::other("path must include a file name")) as BoxError
    })?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let dir = open_ambient_dir(parent)?;
    Ok((dir, file_name))
}

fn resolve_pg_port() -> Result<Option<OsString>, BoxError> {
    if std::env::var_os("PG_PORT").is_some() {
        return Ok(None);
    }

    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|err| Box::new(err) as BoxError)?;
    let port = listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|err| Box::new(err) as BoxError)?;
    drop(listener);

    Ok(Some(OsString::from(port.to_string())))
}

fn locate_pg_worker_near_target() -> Option<OsString> {
    let exe = std::env::current_exe().ok()?;
    let deps_dir = exe.parent()?;
    let target_dir = deps_dir.parent()?;
    let worker_path = target_dir.join("pg_worker");
    worker_path.is_file().then(|| worker_path.into_os_string())
}

fn locate_pg_worker_in_path() -> Option<OsString> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("pg_worker");
        if candidate.is_file() {
            return Some(candidate.into_os_string());
        }
    }
    None
}

fn locate_pg_worker_from_env() -> Option<OsString> {
    let worker = std::env::var_os("PG_EMBEDDED_WORKER")?;
    let file_name = std::path::Path::new(&worker).file_name()?;
    (file_name == OsStr::new("pg_worker")).then_some(worker)
}
