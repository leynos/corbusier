//! Privileged worker helper for embedded `PostgreSQL` test clusters.
//!
//! Usage:
//! ```text
//! pg_worker <operation> <config-path>
//! ```
//!
//! The operation must be `setup`, `start`, or `stop`. The config payload is a
//! JSON-encoded [`pg_embedded_setup_unpriv::worker::WorkerPayload`].

type BoxError = Box<dyn std::error::Error>;

fn main() -> Result<(), BoxError> {
    #[cfg(unix)]
    {
        unix::run()?;
        Ok(())
    }

    #[cfg(not(unix))]
    {
        Err(other_error(
            "pg_worker is only supported on Unix-like hosts.",
        ))
    }
}

#[cfg(unix)]
mod unix {
    //! Unix implementation of the worker helper.

    use super::{BoxError, other_error};
    use pg_embedded_setup_unpriv::worker::{PlainSecret, WorkerPayload};
    use postgresql_embedded::{PostgreSQL, Status};
    use std::env;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::PathBuf;
    use tokio::runtime::Builder;

    enum Operation {
        Setup,
        Start,
        Stop,
    }

    impl Operation {
        fn parse(arg: &OsStr) -> Result<Self, BoxError> {
            match arg.to_string_lossy().as_ref() {
                "setup" => Ok(Self::Setup),
                "start" => Ok(Self::Start),
                "stop" => Ok(Self::Stop),
                other => Err(other_error(format!(
                    "unknown pg_worker operation '{other}'; valid operations are setup, start, and stop"
                ))),
            }
        }
    }

    pub(super) fn run() -> Result<(), BoxError> {
        let mut args = env::args_os();
        let _program = args.next();
        let op_arg = args
            .next()
            .ok_or_else(|| other_error("missing operation argument"))?;
        let operation = Operation::parse(&op_arg)?;
        let config_path = args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| other_error("missing config path argument"))?;
        if let Some(extra) = args.next() {
            let extra_arg = extra.to_string_lossy();
            return Err(other_error(format!(
                "unexpected extra argument: {extra_arg}; expected only operation and config path"
            )));
        }

        let config_bytes = fs::read(&config_path).map_err(|err| Box::new(err) as BoxError)?;
        let payload: WorkerPayload =
            serde_json::from_slice(&config_bytes).map_err(|err| Box::new(err) as BoxError)?;
        let mut settings = payload
            .settings
            .into_settings()
            .map_err(|err| Box::new(err) as BoxError)?;
        settings.temporary = false;

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| Box::new(err) as BoxError)?;

        apply_worker_environment(&payload.environment);
        let mut postgres = PostgreSQL::new(settings);

        runtime.block_on(async {
            match operation {
                Operation::Setup => {
                    ensure_postgres_setup(&mut postgres).await?;
                    std::mem::forget(postgres);
                    Ok::<(), BoxError>(())
                }
                Operation::Start => {
                    ensure_postgres_setup(&mut postgres).await?;
                    ensure_postgres_started(&mut postgres).await?;
                    std::mem::forget(postgres);
                    Ok::<(), BoxError>(())
                }
                Operation::Stop => {
                    postgres
                        .setup()
                        .await
                        .map_err(|err| Box::new(err) as BoxError)?;
                    postgres
                        .stop()
                        .await
                        .map_err(|err| Box::new(err) as BoxError)?;
                    Ok::<(), BoxError>(())
                }
            }
        })?;

        Ok(())
    }

    /// Applies the worker environment overrides to the current process.
    fn apply_worker_environment(environment: &[(String, Option<PlainSecret>)]) {
        for (key, value) in environment {
            unsafe {
                // SAFETY: the worker executes single-threaded, so env mutation is isolated.
                match value {
                    Some(env_value) => env::set_var(key, env_value.expose()),
                    None => env::remove_var(key),
                }
            }
        }
    }

    async fn ensure_postgres_setup(postgres: &mut PostgreSQL) -> Result<(), BoxError> {
        postgres
            .setup()
            .await
            .map_err(|err| Box::new(err) as BoxError)?;

        if matches!(postgres.status(), Status::Started) {
            return Ok(());
        }

        let data_dir = postgres.settings().data_dir.clone();
        if data_dir.join("global").join("pg_filenode.map").exists() {
            return Ok(());
        }

        match fs::remove_dir_all(&data_dir) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(Box::new(err) as BoxError),
        }

        postgres
            .setup()
            .await
            .map_err(|err| Box::new(err) as BoxError)?;
        Ok(())
    }

    async fn ensure_postgres_started(postgres: &mut PostgreSQL) -> Result<(), BoxError> {
        if matches!(postgres.status(), Status::Started) {
            return Ok(());
        }

        postgres
            .start()
            .await
            .map_err(|err| Box::new(err) as BoxError)?;
        Ok(())
    }
}

fn other_error(message: impl Into<String>) -> BoxError {
    Box::new(std::io::Error::other(message.into()))
}
