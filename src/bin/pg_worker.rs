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
    use cap_std::ambient_authority;
    use cap_std::fs::Dir;
    use pg_embedded_setup_unpriv::worker::{PlainSecret, WorkerPayload};
    use postgresql_embedded::{PostgreSQL, Status};
    use std::env;
    use std::ffi::OsStr;
    use std::io::Read;
    use std::path::{Path, PathBuf};
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

        let config_bytes = read_file_bytes(&config_path)?;
        let payload: WorkerPayload = serde_json::from_slice(&config_bytes).map_err(Box::new)?;
        let mut settings = payload.settings.into_settings().map_err(Box::new)?;
        settings.temporary = false;

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(Box::new)?;

        apply_worker_environment(&payload.environment);
        let postgres = PostgreSQL::new(settings);
        runtime.block_on(run_operation(operation, postgres))?;

        Ok(())
    }

    /// Applies the worker environment overrides to the current process.
    fn apply_worker_environment(environment: &[(String, Option<PlainSecret>)]) {
        apply_worker_environment_with(&ProcessEnv, environment);
    }

    trait EnvStore {
        fn set_var(&self, key: &str, value: &str);
        fn remove_var(&self, key: &str);
    }

    struct ProcessEnv;

    impl EnvStore for ProcessEnv {
        fn set_var(&self, key: &str, value: &str) {
            unsafe {
                // SAFETY: the worker runs single-threaded, so env mutations
                // are isolated from concurrent access.
                env::set_var(key, value);
            }
        }

        fn remove_var(&self, key: &str) {
            unsafe {
                // SAFETY: the worker runs single-threaded, so env mutations
                // are isolated from concurrent access.
                env::remove_var(key);
            }
        }
    }

    fn apply_worker_environment_with<E: EnvStore>(
        env_store: &E,
        environment: &[(String, Option<PlainSecret>)],
    ) {
        for (key, value) in environment {
            match value {
                Some(env_value) => env_store.set_var(key, env_value.expose()),
                None => env_store.remove_var(key),
            }
        }
    }

    async fn run_operation(operation: Operation, mut postgres: PostgreSQL) -> Result<(), BoxError> {
        match operation {
            Operation::Setup => {
                ensure_postgres_setup(&mut postgres).await?;
                std::mem::forget(postgres);
            }
            Operation::Start => {
                ensure_postgres_setup(&mut postgres).await?;
                ensure_postgres_started(&mut postgres).await?;
                std::mem::forget(postgres);
            }
            Operation::Stop => {
                postgres.setup().await.map_err(Box::new)?;
                postgres.stop().await.map_err(Box::new)?;
            }
        }
        Ok(())
    }

    async fn ensure_postgres_setup(postgres: &mut PostgreSQL) -> Result<(), BoxError> {
        postgres.setup().await.map_err(Box::new)?;
        if matches!(postgres.status(), Status::Started) {
            return Ok(());
        }

        let data_dir = postgres.settings().data_dir.clone();
        if has_valid_data_dir(&data_dir) {
            return Ok(());
        }

        reset_data_dir(&data_dir)?;
        postgres.setup().await.map_err(Box::new)?;
        Ok(())
    }

    async fn ensure_postgres_started(postgres: &mut PostgreSQL) -> Result<(), BoxError> {
        if matches!(postgres.status(), Status::Started) {
            return Ok(());
        }

        postgres.start().await.map_err(Box::new)?;
        Ok(())
    }

    fn has_valid_data_dir(data_dir: &Path) -> bool {
        data_dir.join("global").join("pg_filenode.map").exists()
    }

    fn reset_data_dir(data_dir: &Path) -> Result<(), BoxError> {
        match remove_dir_all(data_dir) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Box::new(err)),
        }
    }

    fn open_ambient_dir(path: &Path) -> Result<Dir, BoxError> {
        Dir::open_ambient_dir(path, ambient_authority()).map_err(|err| Box::new(err) as BoxError)
    }

    fn open_parent_dir(path: &Path) -> Result<(Dir, &OsStr), BoxError> {
        let file_name = path
            .file_name()
            .ok_or_else(|| other_error("path must include a file name"))?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let dir = open_ambient_dir(parent)?;
        Ok((dir, file_name))
    }

    fn read_file_bytes(path: &Path) -> Result<Vec<u8>, BoxError> {
        let (dir, file_name) = open_parent_dir(path)?;
        let mut file = dir.open(file_name).map_err(Box::new)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(Box::new)?;
        Ok(buffer)
    }

    fn remove_dir_all(path: &Path) -> Result<(), std::io::Error> {
        let (dir, name) =
            open_parent_dir(path).map_err(|err| std::io::Error::other(err.to_string()))?;
        dir.remove_dir_all(name)
    }

    #[cfg(test)]
    mod tests {
        //! Unit tests for the Unix worker helper.

        use super::{EnvStore, Operation, PlainSecret, apply_worker_environment_with, other_error};
        use std::cell::RefCell;
        use std::ffi::OsStr;

        struct RecordingEnv {
            calls: RefCell<Vec<(String, Option<String>)>>,
        }

        impl RecordingEnv {
            fn new() -> Self {
                Self {
                    calls: RefCell::new(Vec::new()),
                }
            }
        }

        impl EnvStore for RecordingEnv {
            fn set_var(&self, key: &str, value: &str) {
                self.calls
                    .borrow_mut()
                    .push((key.to_owned(), Some(value.to_owned())));
            }

            fn remove_var(&self, key: &str) {
                self.calls.borrow_mut().push((key.to_owned(), None));
            }
        }

        #[test]
        fn parse_accepts_supported_operations() {
            assert!(matches!(
                Operation::parse(OsStr::new("setup")),
                Ok(Operation::Setup)
            ));
            assert!(matches!(
                Operation::parse(OsStr::new("start")),
                Ok(Operation::Start)
            ));
            assert!(matches!(
                Operation::parse(OsStr::new("stop")),
                Ok(Operation::Stop)
            ));
        }

        #[test]
        fn parse_rejects_unknown_operation() {
            let result = Operation::parse(OsStr::new("unknown"));
            assert!(result.is_err());
            let message = result.err().expect("error should be present").to_string();
            assert!(message.contains("unknown pg_worker operation"));
        }

        #[test]
        fn apply_worker_environment_records_values() {
            let recorder = RecordingEnv::new();
            let env_vars = vec![
                ("ONE".to_owned(), Some(PlainSecret::from("value"))),
                ("TWO".to_owned(), None),
            ];

            apply_worker_environment_with(&recorder, &env_vars);

            let calls = recorder.calls.borrow();
            assert_eq!(calls.len(), 2);
            assert_eq!(
                calls.as_slice(),
                &[
                    ("ONE".to_owned(), Some("value".to_owned())),
                    ("TWO".to_owned(), None),
                ]
            );
        }

        #[test]
        fn other_error_reports_message() {
            let error = other_error("boom");
            let message = error.to_string();
            assert!(message.contains("boom"));
        }
    }
}

fn other_error(message: impl Into<String>) -> BoxError {
    Box::new(std::io::Error::other(message.into()))
}
