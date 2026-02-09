//! Invokes `PostgreSQL` bootstrap operations inside a privileged worker process.
//!
//! Usage:
//!
//! ```text
//! pg_worker <operation> <config-path>
//! ```
//!
//! The `operation` must be `setup`, `start`, or `stop`. The JSON payload at
//! `config-path` must serialize a [`WorkerPayload`] containing `PostgreSQL`
//! settings and environment overrides. A representative payload is:
//!
//! ```json
//! {
//!   "environment": {
//!     "PG_SUPERUSER": "postgres",
//!     "TZ": null
//!   },
//!   "settings": {
//!     "version": "=16.4.0",
//!     "port": 15433,
//!     "username": "postgres",
//!     "password": "postgres",
//!     "data_dir": "/tmp/data",
//!     "installation_dir": "/tmp/install",
//!     "temporary": false,
//!     "timeout_secs": 30,
//!     "configuration": {
//!       "lc_messages": "C"
//!     },
//!     "trust_installation_dir": true
//!   }
//! }
//! ```
//!
//! The helper mirrors `postgresql_embedded` lifecycle calls while allowing the
//! caller to demote credentials before spawning a child process.

#[cfg(unix)]
use camino::{Utf8Path, Utf8PathBuf};
#[cfg(unix)]
use corbusier::worker::shell_escape;
#[cfg(unix)]
use nix::unistd::{Uid, User, initgroups, setgid, setuid};
#[cfg(unix)]
use pg_embedded_setup_unpriv::worker::{PlainSecret, WorkerPayload};
#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::ffi::{CString, OsStr, OsString};
#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::io::Read;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use thiserror::Error;

#[cfg(unix)]
use pg_embedded_setup_unpriv::ambient_dir_and_path;
#[cfg(unix)]
use postgresql_embedded::{PostgreSQL, Status};
#[cfg(unix)]
use tokio::runtime::Builder;

#[cfg(unix)]
const WORKER_REEXEC_ENV: &str = "PG_WORKER_REEXEC";

/// Boxed error type for the main result.
type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Errors that can occur during worker operations.
#[cfg(unix)]
#[derive(Debug, Error)]
enum WorkerError {
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("failed to read worker config: {0}")]
    ConfigRead(#[source] BoxError),
    #[error("failed to parse worker config: {0}")]
    ConfigParse(#[source] serde_json::Error),
    #[error("settings conversion failed: {0}")]
    SettingsConversion(String),
    #[error("runtime init failed: {0}")]
    RuntimeInit(#[source] std::io::Error),
    #[error("failed to drop privileges: {0}")]
    PrivilegeDrop(String),
    #[error("postgres operation failed: {0}")]
    PostgresOperation(String),
}

#[cfg(unix)]
#[derive(Debug)]
enum Operation {
    Setup,
    Start,
    Stop,
}

#[cfg(unix)]
impl Operation {
    fn parse(arg: &OsStr) -> Result<Self, WorkerError> {
        match arg.to_string_lossy().as_ref() {
            "setup" => Ok(Self::Setup),
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            other => Err(WorkerError::InvalidArgs(format!(
                "unknown operation '{other}'; expected setup, start, or stop"
            ))),
        }
    }
}

#[cfg(unix)]
fn main() -> Result<(), BoxError> {
    let args: Vec<OsString> = env::args_os().collect();
    maybe_reexec_as_nobody(&args)?;
    run_worker(args.into_iter()).map_err(Into::into)
}

#[cfg(unix)]
fn run_worker(args: impl Iterator<Item = OsString>) -> Result<(), WorkerError> {
    let (operation, config_path) = parse_args(args)?;
    let payload = load_payload(&config_path)?;
    drop_privileges_if_root("nobody")?;
    let settings = payload
        .settings
        .into_settings()
        .map_err(|e| WorkerError::SettingsConversion(e.to_string()))?;

    let runtime = build_runtime()?;
    apply_worker_environment(&payload.environment);
    let mut pg = Some(PostgreSQL::new(settings));
    runtime.block_on(async {
        match operation {
            Operation::Setup => {
                let pg_handle = pg.as_mut().ok_or_else(|| {
                    WorkerError::PostgresOperation("pg handle missing during setup".into())
                })?;
                ensure_postgres_setup(pg_handle).await
            }
            Operation::Start => {
                let pg_handle = pg.as_mut().ok_or_else(|| {
                    WorkerError::PostgresOperation("pg handle missing during start".into())
                })?;
                ensure_postgres_started(pg_handle).await?;

                if let Some(pg_instance) = pg.take() {
                    // Intentionally leak to keep PostgreSQL running after worker exit.
                    let _leaked = std::mem::ManuallyDrop::new(pg_instance);
                }
                Ok(())
            }
            Operation::Stop => execute_stop(&mut pg).await,
        }
    })?;
    Ok(())
}

#[cfg(unix)]
fn maybe_reexec_as_nobody(args: &[OsString]) -> Result<(), WorkerError> {
    if !Uid::effective().is_root() || env::var_os(WORKER_REEXEC_ENV).is_some() {
        return Ok(());
    }

    let exe = env::current_exe().map_err(WorkerError::RuntimeInit)?;
    let status = match Command::new("runuser")
        .arg("-u")
        .arg("nobody")
        .arg("--")
        .arg(&exe)
        .args(args.iter().skip(1))
        .env(WORKER_REEXEC_ENV, "1")
        .status()
    {
        Ok(status) => status,
        Err(err) if err.kind() == io::ErrorKind::NotFound => run_via_su(&exe, args)?,
        Err(err) => return Err(WorkerError::PrivilegeDrop(err.to_string())),
    };

    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(unix)]
fn run_via_su(
    exe: &std::path::Path,
    args: &[OsString],
) -> Result<std::process::ExitStatus, WorkerError> {
    let mut command = format!(
        "{WORKER_REEXEC_ENV}=1 exec {}",
        shell_escape(exe.to_string_lossy().as_ref())
    );
    for arg in args.iter().skip(1) {
        command.push(' ');
        command.push_str(&shell_escape(arg.to_string_lossy().as_ref()));
    }

    Command::new("su")
        .arg("-s")
        .arg("/bin/sh")
        .arg("nobody")
        .arg("-c")
        .arg(command)
        .status()
        .map_err(|err| WorkerError::PrivilegeDrop(err.to_string()))
}

#[cfg(unix)]
fn parse_args(
    mut args: impl Iterator<Item = OsString>,
) -> Result<(Operation, Utf8PathBuf), WorkerError> {
    let _program = args.next();
    let operation = args
        .next()
        .ok_or_else(|| WorkerError::InvalidArgs("missing operation argument".into()))
        .and_then(|arg| Operation::parse(&arg))?;
    let config_path_buf = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| WorkerError::InvalidArgs("missing config path argument".into()))?;
    let config_path = Utf8PathBuf::from_path_buf(config_path_buf).map_err(|p| {
        WorkerError::InvalidArgs(format!("config path is not valid UTF-8: {}", p.display()))
    })?;
    if let Some(extra) = args.next() {
        let extra_arg = extra.to_string_lossy();
        return Err(WorkerError::InvalidArgs(format!(
            "unexpected extra argument: {extra_arg}"
        )));
    }
    Ok((operation, config_path))
}

#[cfg(unix)]
fn load_payload(config_path: &Utf8Path) -> Result<WorkerPayload, WorkerError> {
    let config_bytes = read_config_file(config_path).map_err(WorkerError::ConfigRead)?;
    serde_json::from_slice(&config_bytes).map_err(WorkerError::ConfigParse)
}

#[cfg(unix)]
fn read_config_file(path: &Utf8Path) -> Result<Vec<u8>, BoxError> {
    let (dir, relative) = ambient_dir_and_path(path)?;
    let mut file = dir.open(relative.as_std_path())?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(unix)]
fn drop_privileges_if_root(username: &str) -> Result<(), WorkerError> {
    if !Uid::effective().is_root() {
        return Ok(());
    }

    let user = User::from_name(username)
        .map_err(|err| WorkerError::PrivilegeDrop(err.to_string()))?
        .ok_or_else(|| WorkerError::PrivilegeDrop(format!("user '{username}' not found")))?;

    let user_cstr = CString::new(user.name.clone()).map_err(|err| {
        WorkerError::PrivilegeDrop(format!("invalid user name for initgroups: {err}"))
    })?;
    initgroups(&user_cstr, user.gid).map_err(|err| WorkerError::PrivilegeDrop(err.to_string()))?;
    setgid(user.gid).map_err(|err| WorkerError::PrivilegeDrop(err.to_string()))?;
    setuid(user.uid).map_err(|err| WorkerError::PrivilegeDrop(err.to_string()))?;

    // SAFETY: the worker executes single-threaded and owns env var changes.
    unsafe {
        env::set_var("HOME", user.dir);
        env::set_var("USER", user.name.clone());
        env::set_var("LOGNAME", user.name);
    }

    Ok(())
}

#[cfg(unix)]
fn build_runtime() -> Result<tokio::runtime::Runtime, WorkerError> {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(WorkerError::RuntimeInit)
}

#[cfg(unix)]
fn apply_worker_environment(environment: &[(String, Option<PlainSecret>)]) {
    for (key, value) in environment {
        match value {
            Some(plain) => {
                // SAFETY: tests control the worker lifecycle and no other threads mutate vars.
                unsafe {
                    env::set_var(key, plain.expose());
                }
            }
            None => {
                // SAFETY: tests control the worker lifecycle and no other threads mutate vars.
                unsafe {
                    env::remove_var(key);
                }
            }
        }
    }
}

#[cfg(unix)]
async fn ensure_postgres_setup(postgres: &mut PostgreSQL) -> Result<(), WorkerError> {
    postgres
        .setup()
        .await
        .map_err(|err| WorkerError::PostgresOperation(err.to_string()))?;
    ensure_postgres_started(postgres).await?;
    Ok(())
}

#[cfg(unix)]
async fn ensure_postgres_started(postgres: &mut PostgreSQL) -> Result<(), WorkerError> {
    if matches!(postgres.status(), Status::Started) {
        return Ok(());
    }

    postgres
        .start()
        .await
        .map_err(|err| WorkerError::PostgresOperation(err.to_string()))
}

#[cfg(unix)]
async fn execute_stop(pg: &mut Option<PostgreSQL>) -> Result<(), WorkerError> {
    let Some(instance) = pg.take() else {
        return Err(WorkerError::PostgresOperation(
            "pg handle missing during stop".into(),
        ));
    };

    instance
        .stop()
        .await
        .map_err(|err| WorkerError::PostgresOperation(err.to_string()))
}

#[cfg(not(unix))]
fn main() -> Result<(), BoxError> {
    Err("pg_worker is not supported on non-Unix platforms".into())
}
