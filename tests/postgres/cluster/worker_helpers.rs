//! Worker binary discovery and preparation helpers.

use super::BoxError;
use super::fs_utils::open_parent_dir;
use camino::{Utf8Path, Utf8PathBuf};
#[cfg(unix)]
use cap_std::fs::Permissions;
#[cfg(unix)]
use cap_std::fs::PermissionsExt;
use std::sync::OnceLock;

static WORKER_CACHE: OnceLock<Utf8PathBuf> = OnceLock::new();

pub(super) fn locate_pg_worker_path() -> Option<Utf8PathBuf> {
    crate::test_helpers::locate_pg_worker_path()
}

pub(super) fn prepare_pg_worker(worker: &Utf8Path) -> Result<Utf8PathBuf, BoxError> {
    if let Some(cached) = WORKER_CACHE.get() {
        return Ok(cached.clone());
    }

    let temp_dir = Utf8PathBuf::from(std::env::temp_dir().to_string_lossy().into_owned());
    let destination_path = temp_dir.join(format!("pg_worker_{pid}", pid = std::process::id()));
    let (source_dir, source_name_str) = open_parent_dir(worker)?;
    let (destination_dir, destination_name_str) = open_parent_dir(&destination_path)?;
    let source_name = Utf8Path::new(source_name_str);
    let destination_name = Utf8Path::new(destination_name_str);

    match destination_dir.remove_file(destination_name) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(Box::new(err) as BoxError),
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

    if WORKER_CACHE.set(destination_path.clone()).is_err() {
        // Another test stored the prepared worker path first.
    }
    Ok(destination_path)
}

#[cfg(test)]
mod tests {
    use super::{BoxError, locate_pg_worker_path, prepare_pg_worker};
    use crate::test_helpers::EnvVarGuard;
    use camino::{Utf8Path, Utf8PathBuf};
    use cap_std::ambient_authority;
    use cap_std::fs_utf8::Dir;
    use std::ffi::OsString;
    use std::io::{self, Write};

    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt as StdPermissionsExt;

    fn create_temp_dir(prefix: &str) -> Result<Utf8PathBuf, io::Error> {
        let base = Utf8PathBuf::from(std::env::temp_dir().to_string_lossy().into_owned());
        let dir_name = format!("{prefix}_{}", uuid::Uuid::new_v4());
        let base_dir = Dir::open_ambient_dir(&base, ambient_authority())?;
        base_dir.create_dir(&dir_name)?;
        Ok(base.join(dir_name))
    }

    fn write_worker_script(dir: &Utf8Path, name: &str) -> Result<Utf8PathBuf, io::Error> {
        let dir_handle = Dir::open_ambient_dir(dir, ambient_authority())?;
        let mut file = dir_handle.create(name)?;
        file.write_all(b"#!/bin/sh\nexit 0\n")?;
        Ok(dir.join(name))
    }

    #[test]
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test uses assertions for verification while returning Result for error propagation"
    )]
    fn env_var_takes_precedence_over_other_discovery() -> Result<(), BoxError> {
        let env_dir = create_temp_dir("pg_worker_env")?;
        let env_worker = write_worker_script(&env_dir, "pg_worker")?;
        let path_dir = create_temp_dir("pg_worker_path")?;
        let _path_worker = write_worker_script(&path_dir, "pg_worker")?;

        let guard = EnvVarGuard::set_many(&[
            (
                OsString::from("CARGO_BIN_EXE_pg_worker"),
                Some(OsString::from(env_worker.as_str())),
            ),
            (
                OsString::from("PATH"),
                Some(OsString::from(path_dir.as_str())),
            ),
            (OsString::from("PG_EMBEDDED_WORKER"), None),
        ]);

        let located = locate_pg_worker_path().ok_or_else(|| {
            Box::new(io::Error::new(
                io::ErrorKind::NotFound,
                "expected worker path from CARGO_BIN_EXE_pg_worker",
            )) as BoxError
        })?;
        assert_eq!(
            located, env_worker,
            "CARGO_BIN_EXE_pg_worker should take precedence over PATH discovery",
        );
        drop(guard);
        Ok(())
    }

    #[test]
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test uses assertions for verification while returning Result for error propagation"
    )]
    fn falls_back_to_path_when_env_unset() -> Result<(), BoxError> {
        let path_dir = create_temp_dir("pg_worker_fallback")?;
        let _path_worker = write_worker_script(&path_dir, "pg_worker")?;

        let guard = EnvVarGuard::set_many(&[
            (OsString::from("CARGO_BIN_EXE_pg_worker"), None),
            (OsString::from("PG_EMBEDDED_WORKER"), None),
            (
                OsString::from("PATH"),
                Some(OsString::from(path_dir.as_str())),
            ),
        ]);

        let located = locate_pg_worker_path().ok_or_else(|| {
            Box::new(io::Error::new(
                io::ErrorKind::NotFound,
                "expected worker path from fallback discovery",
            )) as BoxError
        })?;
        assert!(
            located.is_file(),
            "expected fallback to locate a worker binary, got {located}"
        );
        drop(guard);
        Ok(())
    }

    #[test]
    #[expect(
        clippy::panic_in_result_fn,
        reason = "Test uses assertions for verification while returning Result for error propagation"
    )]
    fn prepare_pg_worker_is_idempotent_and_sets_permissions() -> Result<(), BoxError> {
        let source_dir = create_temp_dir("pg_worker_source")?;
        let worker_path = write_worker_script(&source_dir, "pg_worker")?;

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(worker_path.as_std_path())?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(worker_path.as_std_path(), perms)?;
        }

        let prepared = prepare_pg_worker(&worker_path)?;
        let prepared_second = prepare_pg_worker(&worker_path)?;
        assert_eq!(
            prepared, prepared_second,
            "prepare_pg_worker should reuse the cached worker path",
        );

        #[cfg(unix)]
        {
            let meta = fs::metadata(prepared.as_std_path())?;
            let mode = meta.permissions().mode();
            assert!(
                mode & 0o111 != 0,
                "expected prepared worker to be executable, mode {mode:o}",
            );
        }
        Ok(())
    }
}
