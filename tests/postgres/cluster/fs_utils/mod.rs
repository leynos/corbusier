//! Filesystem helpers for `PostgreSQL` test clusters.

use super::BoxError;
use camino::{Utf8Path, Utf8PathBuf};
use cap_std::ambient_authority;
use cap_std::fs_utf8::{Dir, File, OpenOptions, Permissions};
use postgresql_embedded::Settings;
use std::io::{self, Read};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TIMEZONE_STAGE: AtomicU64 = AtomicU64::new(0);

/// Opens an explicit directory capability for test-cluster filesystem work.
pub(super) fn open_ambient_dir(path: &Utf8Path) -> Result<Dir, BoxError> {
    Dir::open_ambient_dir(path, ambient_authority()).map_err(|err| Box::new(err) as BoxError)
}

/// Opens a file's parent capability and returns its relative basename.
pub(super) fn open_parent_dir(path: &Utf8Path) -> Result<(Dir, &str), BoxError> {
    let file_name = path.file_name().ok_or_else(|| {
        Box::new(std::io::Error::other("path must include a file name")) as BoxError
    })?;
    let parent = path.parent().unwrap_or_else(|| Utf8Path::new("."));
    let dir = open_ambient_dir(parent)?;
    Ok((dir, file_name))
}

/// Restores `PostgreSQL`'s standard UTC alias when the bundled database omits it.
///
/// Call after setup and before server start in either startup path. For an
/// installation containing only `UCT`, success leaves both names available;
/// an existing `UTC` file is preserved, including one installed concurrently.
///
/// # Errors
///
/// Returns filesystem errors without leaving a partially copied `UTC` alias.
pub(super) fn ensure_utc_timezone_file(settings: &Settings) -> Result<(), BoxError> {
    let installation_dir = Utf8Path::from_path(&settings.installation_dir).ok_or_else(|| {
        Box::new(std::io::Error::other(format!(
            "PostgreSQL installation directory is not UTF-8: {}",
            settings.installation_dir.display()
        ))) as BoxError
    })?;
    let timezone_dir = installation_dir.join("share").join("timezone");
    let dir = open_ambient_dir(&timezone_dir)?;

    match dir.metadata(Utf8Path::new("UTC")) {
        Ok(_) => return Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(Box::new(err) as BoxError),
    }

    let mut source = dir
        .open(Utf8Path::new("UCT"))
        .map_err(|err| Box::new(err) as BoxError)?;
    let source_permissions = source
        .metadata()
        .map_err(|err| Box::new(err) as BoxError)?
        .permissions();
    publish_utc_alias(&dir, &mut source, source_permissions)
        .map_err(|err| Box::new(err) as BoxError)
}

/// Copies a timezone source privately, then publishes its complete UTC alias.
///
/// This helper belongs only to the installation repair above and its tests;
/// accepting a reader permits deterministic coverage of interrupted copies.
/// Source permissions are restored before publication so a restrictive caller
/// umask cannot prevent the `PostgreSQL` worker from reading the completed alias.
fn publish_utc_alias(
    timezone_dir: &Dir,
    source: &mut impl Read,
    source_permissions: Permissions,
) -> io::Result<()> {
    let (staging_name, mut staging_file) = create_timezone_stage(timezone_dir)?;
    let result = io::copy(source, &mut staging_file)
        .and_then(|_| staging_file.set_permissions(source_permissions))
        .and_then(|()| staging_file.sync_all())
        .and_then(
            |()| match timezone_dir.hard_link(&staging_name, timezone_dir, "UTC") {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
                Err(error) => Err(error),
            },
        );
    // Close the handle before unlinking on platforms that reject open-file removal.
    drop(staging_file);
    let cleanup = timezone_dir.remove_file(&staging_name);
    match (result, cleanup) {
        (Ok(()), cleanup_result) => cleanup_result,
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(cleanup_error)) => Err(io::Error::new(
            error.kind(),
            format!("{error}; failed to remove {staging_name}: {cleanup_error}"),
        )),
    }
}

/// Exclusively creates a same-directory staging file for one repair attempt.
///
/// Process identity and a counter avoid live-name collisions; exclusive creation
/// also skips stale files left by an earlier process with a reused process ID.
fn create_timezone_stage(timezone_dir: &Dir) -> io::Result<(Utf8PathBuf, File)> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    loop {
        let sequence = NEXT_TIMEZONE_STAGE.fetch_add(1, Ordering::Relaxed);
        let name = Utf8PathBuf::from(format!(".UTC-{}-{sequence}.tmp", std::process::id()));
        match timezone_dir.open_with(&name, &options) {
            Ok(file) => return Ok((name, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

/// Refreshes credentials after another process creates the shared password file.
pub(super) fn sync_password_from_file(settings: &mut Settings) -> Result<(), BoxError> {
    let password_path_string = settings.password_file.to_string_lossy();
    // settings.password_file is expected to be UTF-8, so lossy conversion is
    // acceptable for Utf8Path::new(password_path_string.as_ref()).
    let password_path = Utf8Path::new(password_path_string.as_ref());
    let (dir, file_name_str) = open_parent_dir(password_path)?;
    let file_name = Utf8Path::new(file_name_str);
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

/// Reads the server-selected port from the shared cluster's PID file.
pub(super) fn sync_port_from_pid(settings: &mut Settings) -> Result<(), BoxError> {
    let data_dir_string = settings.data_dir.to_string_lossy();
    let data_dir_path = Utf8Path::new(data_dir_string.as_ref());
    let data_dir_handle = open_ambient_dir(data_dir_path)?;
    let contents = match data_dir_handle.read_to_string(Utf8Path::new("postmaster.pid")) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Box::new(err) as BoxError),
    };

    let port_line = contents.lines().nth(3).map(str::trim).ok_or_else(|| {
        Box::new(std::io::Error::other("postmaster.pid missing port line")) as BoxError
    })?;
    let port = port_line.parse::<u16>().map_err(|err| {
        Box::new(std::io::Error::other(format!(
            "failed to parse postmaster.pid port: {err}"
        ))) as BoxError
    })?;
    settings.port = port;
    Ok(())
}

/// Remove a stale `postmaster.pid` if the PID no longer exists on Linux.
///
/// On non-Linux platforms, the PID is parsed but no `/proc` check is attempted.
pub(super) fn cleanup_stale_postmaster_pid(settings: &Settings) -> Result<(), BoxError> {
    let data_dir_string = settings.data_dir.to_string_lossy();
    let data_dir_path = Utf8Path::new(data_dir_string.as_ref());
    let data_dir_handle = open_ambient_dir(data_dir_path)?;
    let pid_contents = match data_dir_handle.read_to_string(Utf8Path::new("postmaster.pid")) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Box::new(err) as BoxError),
    };
    let pid_line = pid_contents
        .lines()
        .next()
        .ok_or_else(|| Box::new(std::io::Error::other("postmaster.pid missing pid")) as BoxError)?;
    let pid = pid_line.parse::<i32>().map_err(|err| {
        Box::new(std::io::Error::other(format!(
            "failed to parse postmaster.pid pid: {err}"
        ))) as BoxError
    })?;

    if cfg!(target_os = "linux") {
        let proc_dir = open_ambient_dir(Utf8Path::new("/proc"))?;
        let pid_string = pid.to_string();
        if proc_dir.metadata(Utf8Path::new(&pid_string)).is_err() {
            data_dir_handle
                .remove_file(Utf8Path::new("postmaster.pid"))
                .map_err(|err| Box::new(err) as BoxError)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
