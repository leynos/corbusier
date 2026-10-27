//! Regression coverage for complete, concurrent publication of UTC timezone data.

use super::test_support::{TemporaryInstall, installation, publish_test_source, timezone_files};
use super::{BoxError, ensure_utc_timezone_file};
use crate::postgres::cluster::{PostgresCluster, postgres_cluster};
#[cfg(unix)]
use cap_std::fs::PermissionsExt;
use diesel::{Connection, PgConnection, QueryableByName, RunQueryDsl};
use rstest::rstest;
use std::collections::BTreeSet;
use std::io::{self, Cursor, Read};
use std::sync::{Arc, Barrier, mpsc};
use std::thread;

/// Creates a missing alias and preserves a previously installed alias.
#[rstest]
#[case::missing_alias(None)]
#[case::existing_alias(Some("existing UTC"))]
fn utc_alias_is_installed_once(
    installation: Result<TemporaryInstall, BoxError>,
    #[case] existing_alias: Option<&str>,
) -> Result<(), BoxError> {
    let test_installation = installation?;
    let directory = test_installation.timezone_dir()?;
    directory.write("UCT", b"source timezone")?;
    if let Some(contents) = existing_alias {
        directory.write("UTC", contents.as_bytes())?;
    }

    ensure_utc_timezone_file(&test_installation.settings())?;

    let actual_alias = directory.read_to_string("UTC")?;
    let expected_alias = existing_alias.unwrap_or("source timezone");
    if actual_alias != expected_alias {
        return Err(
            format!("UTC contents: expected {expected_alias:?}, got {actual_alias:?}").into(),
        );
    }
    let files = timezone_files(&directory)?;
    let expected_files = BTreeSet::from(["UCT".to_owned(), "UTC".to_owned()]);
    if files != expected_files {
        return Err(format!("timezone files: expected {expected_files:?}, got {files:?}").into());
    }
    Ok(())
}

/// Preserves source access attributes independently of staging-file defaults.
#[rstest]
fn utc_alias_preserves_source_permissions(
    installation: Result<TemporaryInstall, BoxError>,
) -> Result<(), BoxError> {
    let test_installation = installation?;
    let directory = test_installation.timezone_dir()?;
    directory.write("UCT", b"readable timezone")?;
    let original_permissions = directory.metadata("UCT")?.permissions();
    let mut source_permissions = original_permissions.clone();
    source_permissions.set_readonly(true);
    directory.set_permissions("UCT", source_permissions.clone())?;

    ensure_utc_timezone_file(&test_installation.settings())?;
    let installed_permissions = directory.metadata("UTC")?.permissions();
    // Restore attributes before checking results so Windows can remove both files.
    directory.set_permissions("UCT", original_permissions.clone())?;
    directory.set_permissions("UTC", original_permissions)?;

    if installed_permissions.readonly() != source_permissions.readonly() {
        return Err("UTC did not preserve the source's read-only attribute".into());
    }
    #[cfg(unix)]
    if installed_permissions.mode() != source_permissions.mode() {
        return Err(format!(
            "UTC permissions: expected {:o}, got {:o}",
            source_permissions.mode(),
            installed_permissions.mode()
        )
        .into());
    }
    Ok(())
}

/// Propagates absent source and directory errors without publishing an alias.
#[rstest]
#[case::missing_uct(false)]
#[case::missing_timezone_directory(true)]
fn missing_timezone_inputs_are_errors(
    installation: Result<TemporaryInstall, BoxError>,
    #[case] should_remove_directory: bool,
) -> Result<(), BoxError> {
    let test_installation = installation?;
    if should_remove_directory {
        test_installation
            .directory()?
            .remove_dir_all("share/timezone")?;
    }

    let error = ensure_utc_timezone_file(&test_installation.settings())
        .err()
        .ok_or_else(|| io::Error::other("missing timezone input unexpectedly passed repair"))?;

    let error_kind = error.downcast_ref::<io::Error>().map(io::Error::kind);
    if error_kind != Some(io::ErrorKind::NotFound) {
        return Err(format!("missing timezone input: expected NotFound, got {error:?}").into());
    }
    if !should_remove_directory && !timezone_files(&test_installation.timezone_dir()?)?.is_empty() {
        return Err("a missing source left a UTC or staging file".into());
    }
    Ok(())
}

/// Injects a read error after a chained reader has supplied a partial prefix.
struct FailingReader;

impl Read for FailingReader {
    /// Reports a deterministic interrupted copy without permission assumptions.
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "interrupted timezone copy",
        ))
    }
}

/// Removes a partially populated staging file when copying fails.
#[rstest]
fn failed_copy_cleans_staging_file(
    installation: Result<TemporaryInstall, BoxError>,
) -> Result<(), BoxError> {
    let test_installation = installation?;
    let directory = test_installation.timezone_dir()?;
    let mut source = Cursor::new(b"partial timezone").chain(FailingReader);

    let error = publish_test_source(&directory, &mut source)
        .err()
        .ok_or_else(|| io::Error::other("an interrupted source was unexpectedly published"))?;

    if error.kind() != io::ErrorKind::UnexpectedEof {
        return Err(format!("copy error: expected UnexpectedEof, got {error:?}").into());
    }
    let files = timezone_files(&directory)?;
    if !files.is_empty() {
        return Err(format!("failed copy left files behind: {files:?}").into());
    }
    Ok(())
}

/// Preserves a winner installed after the caller's initial existence check.
#[rstest]
fn concurrent_winner_is_preserved(
    installation: Result<TemporaryInstall, BoxError>,
) -> Result<(), BoxError> {
    let test_installation = installation?;
    let directory = test_installation.timezone_dir()?;
    directory.write("UTC", b"concurrent winner")?;

    publish_test_source(&directory, &mut Cursor::new(b"losing source"))?;

    let winner = directory.read_to_string("UTC")?;
    if winner != "concurrent winner" {
        return Err(format!("concurrent winner was replaced: {winner:?}").into());
    }
    let files = timezone_files(&directory)?;
    if files != BTreeSet::from(["UTC".to_owned()]) {
        return Err(format!("losing publication left unexpected files: {files:?}").into());
    }
    Ok(())
}

/// Allows simultaneous publishers to leave one complete alias and no scratch.
#[rstest]
fn simultaneous_publishers_leave_one_complete_alias(
    installation: Result<TemporaryInstall, BoxError>,
) -> Result<(), BoxError> {
    let test_installation = installation?;
    let directory = Arc::new(test_installation.timezone_dir()?);
    let payloads: Vec<_> = (0..4)
        .map(|index| format!("timezone-{index}").repeat(8192).into_bytes())
        .collect();
    let barrier = Arc::new(Barrier::new(payloads.len()));
    thread::scope(|scope| {
        let handles: Vec<_> = payloads
            .iter()
            .map(|payload| {
                let worker_directory = Arc::clone(&directory);
                let worker_barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    worker_barrier.wait();
                    publish_test_source(&worker_directory, &mut Cursor::new(payload))
                })
            })
            .collect();
        for handle in handles {
            handle
                .join()
                .map_err(|_| io::Error::other("timezone publisher panicked"))??;
        }
        Ok::<(), io::Error>(())
    })?;

    let published = directory.read("UTC")?;
    if !payloads.contains(&published) {
        return Err(format!(
            "UTC does not match a complete source ({} bytes)",
            published.len()
        )
        .into());
    }
    let files = timezone_files(&directory)?;
    if files != BTreeSet::from(["UTC".to_owned()]) {
        return Err(format!("simultaneous publishers left unexpected files: {files:?}").into());
    }
    Ok(())
}

/// Blocks the end of a copy so the test can inspect its publication boundary.
struct PausedEof {
    copying: mpsc::SyncSender<()>,
    resume: mpsc::Receiver<()>,
}

impl Read for PausedEof {
    /// Waits until the caller has inspected the partially completed attempt.
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        self.copying.send(()).map_err(io::Error::other)?;
        self.resume.recv().map_err(io::Error::other)?;
        Ok(0)
    }
}

/// Keeps UTC absent until the entire source has been copied successfully.
#[rstest]
fn incomplete_copy_is_not_visible(
    installation: Result<TemporaryInstall, BoxError>,
) -> Result<(), BoxError> {
    let test_installation = installation?;
    let directory = Arc::new(test_installation.timezone_dir()?);
    let (copying, waiting) = mpsc::sync_channel(0);
    let (resume, paused) = mpsc::sync_channel(0);
    let writer_dir = Arc::clone(&directory);
    let writer = thread::spawn(move || {
        let mut source = Cursor::new(b"complete timezone").chain(PausedEof {
            copying,
            resume: paused,
        });
        publish_test_source(&writer_dir, &mut source)
    });
    waiting.recv()?;
    let alias_is_missing = directory
        .metadata("UTC")
        .is_err_and(|error| error.kind() == io::ErrorKind::NotFound);
    resume.send(())?;
    writer
        .join()
        .map_err(|_| io::Error::other("timezone publisher panicked"))??;

    if !alias_is_missing {
        return Err("UTC became visible before copy completion".into());
    }
    let published = directory.read_to_string("UTC")?;
    if published != "complete timezone" {
        return Err(format!("completed UTC contents are incorrect: {published:?}").into());
    }
    Ok(())
}

/// Connects through the public cluster fixture with an explicit UTC setting.
#[rstest]
fn postgres_connection_accepts_utc(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    #[derive(QueryableByName)]
    struct TimezoneRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        timezone: String,
    }

    let cluster = postgres_cluster?;
    let url = format!(
        "{}?options=-c%20TimeZone%3DUTC",
        cluster.connection().database_url("postgres")
    );
    let mut connection = PgConnection::establish(&url)?;
    let row: TimezoneRow = diesel::sql_query("SELECT current_setting('TimeZone') AS timezone")
        .get_result(&mut connection)?;
    if row.timezone != "UTC" {
        return Err(format!("connection timezone: expected UTC, got {:?}", row.timezone).into());
    }
    Ok(())
}
