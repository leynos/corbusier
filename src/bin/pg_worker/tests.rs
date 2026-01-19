//! Unit tests for the Unix worker helper.

use super::{
    BoxError, EnvStore, Operation, PlainSecret, PostgresLifecycle, Status,
    apply_worker_environment_with, ensure_postgres_setup, ensure_postgres_started,
    has_valid_data_dir, open_ambient_dir, other_error, remove_dir_all,
};
use async_trait::async_trait;
use cap_std::ambient_authority;
use cap_std::fs::Dir;
use std::cell::RefCell;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

struct RecordingEnv {
    calls: RefCell<Vec<(String, Option<String>)>>,
}

impl RecordingEnv {
    const fn new() -> Self {
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

struct FakePostgres {
    status: Status,
    status_after_setup: Option<Status>,
    data_dir: PathBuf,
    setup_calls: usize,
    start_calls: usize,
}

impl FakePostgres {
    const fn new(status: Status, data_dir: PathBuf) -> Self {
        Self {
            status,
            status_after_setup: None,
            data_dir,
            setup_calls: 0,
            start_calls: 0,
        }
    }

    const fn with_status_after_setup(mut self, status: Status) -> Self {
        self.status_after_setup = Some(status);
        self
    }
}

#[async_trait]
impl PostgresLifecycle for FakePostgres {
    async fn setup(&mut self) -> Result<(), BoxError> {
        self.setup_calls += 1;
        if let Some(status) = self.status_after_setup {
            self.status = status;
        }
        Ok(())
    }

    async fn start(&mut self) -> Result<(), BoxError> {
        self.start_calls += 1;
        self.status = Status::Started;
        Ok(())
    }

    fn status(&self) -> Status {
        self.status
    }

    fn data_dir(&self) -> &Path {
        self.data_dir.as_path()
    }
}

fn make_temp_data_dir() -> Result<PathBuf, BoxError> {
    let name = format!("pg_worker_test_{}", uuid::Uuid::new_v4());
    let temp_dir = std::env::temp_dir();
    let dir = Dir::open_ambient_dir(&temp_dir, ambient_authority()).map_err(Box::new)?;
    dir.create_dir(&name).map_err(Box::new)?;
    Ok(temp_dir.join(name))
}

fn create_valid_marker(data_dir: &Path) -> Result<(), BoxError> {
    let dir = open_ambient_dir(data_dir)?;
    dir.create_dir_all("global").map_err(Box::new)?;
    let _file = dir.create("global/pg_filenode.map").map_err(Box::new)?;
    Ok(())
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

#[tokio::test]
async fn ensure_postgres_started_skips_when_started() -> Result<(), BoxError> {
    let data_dir = make_temp_data_dir()?;
    let mut postgres = FakePostgres::new(Status::Started, data_dir.clone());

    ensure_postgres_started(&mut postgres).await?;

    if postgres.start_calls != 0 {
        return Err(other_error(format!(
            "expected 0 start calls, got {}",
            postgres.start_calls
        )));
    }
    remove_dir_all(&data_dir).map_err(Box::new)?;
    Ok(())
}

#[tokio::test]
async fn ensure_postgres_started_runs_when_stopped() -> Result<(), BoxError> {
    let data_dir = make_temp_data_dir()?;
    let mut postgres = FakePostgres::new(Status::Stopped, data_dir.clone());

    ensure_postgres_started(&mut postgres).await?;

    if postgres.start_calls != 1 {
        return Err(other_error(format!(
            "expected 1 start call, got {}",
            postgres.start_calls
        )));
    }
    remove_dir_all(&data_dir).map_err(Box::new)?;
    Ok(())
}

#[tokio::test]
async fn ensure_postgres_setup_returns_when_started() -> Result<(), BoxError> {
    let data_dir = make_temp_data_dir()?;
    let mut postgres = FakePostgres::new(Status::Started, data_dir.clone());

    ensure_postgres_setup(&mut postgres).await?;

    if postgres.setup_calls != 1 {
        return Err(other_error(format!(
            "expected 1 setup call, got {}",
            postgres.setup_calls
        )));
    }
    remove_dir_all(&data_dir).map_err(Box::new)?;
    Ok(())
}

#[tokio::test]
async fn ensure_postgres_setup_skips_reset_when_valid() -> Result<(), BoxError> {
    let data_dir = make_temp_data_dir()?;
    create_valid_marker(&data_dir)?;
    let mut postgres = FakePostgres::new(Status::Stopped, data_dir.clone())
        .with_status_after_setup(Status::Stopped);

    ensure_postgres_setup(&mut postgres).await?;

    if postgres.setup_calls != 1 {
        return Err(other_error(format!(
            "expected 1 setup call, got {}",
            postgres.setup_calls
        )));
    }
    if !has_valid_data_dir(&data_dir) {
        return Err(other_error("expected valid data directory"));
    }
    remove_dir_all(&data_dir).map_err(Box::new)?;
    Ok(())
}

#[tokio::test]
async fn ensure_postgres_setup_resets_invalid_data_dir() -> Result<(), BoxError> {
    let data_dir = make_temp_data_dir()?;
    let mut postgres = FakePostgres::new(Status::Stopped, data_dir.clone())
        .with_status_after_setup(Status::Stopped);

    ensure_postgres_setup(&mut postgres).await?;

    if postgres.setup_calls != 2 {
        return Err(other_error(format!(
            "expected 2 setup calls, got {}",
            postgres.setup_calls
        )));
    }
    if data_dir.exists() {
        return Err(other_error("expected data directory to be removed"));
    }
    Ok(())
}
