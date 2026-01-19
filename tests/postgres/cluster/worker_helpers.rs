//! Worker binary discovery and preparation helpers.

use super::BoxError;
use super::fs_utils::open_parent_dir;
use camino::{Utf8Path, Utf8PathBuf};
#[cfg(unix)]
use cap_std::fs::Permissions;
#[cfg(unix)]
use cap_std::fs::PermissionsExt;
use std::ffi::OsStr;
use std::sync::OnceLock;

static WORKER_CACHE: OnceLock<Utf8PathBuf> = OnceLock::new();

pub(super) fn locate_pg_worker_path() -> Option<Utf8PathBuf> {
    std::env::var_os("CARGO_BIN_EXE_pg_worker")
        .map(|path| utf8_path_from_os(path.as_os_str()))
        .or_else(locate_pg_worker_near_target)
        .or_else(locate_pg_worker_in_path)
        .or_else(locate_pg_worker_from_env)
}

pub(super) fn prepare_pg_worker(worker: &Utf8Path) -> Result<Utf8PathBuf, BoxError> {
    if let Some(cached) = WORKER_CACHE.get() {
        return Ok(cached.clone());
    }

    let temp_dir = utf8_path_from_os(std::env::temp_dir().as_os_str());
    let destination_path = temp_dir.join(format!("pg_worker_{pid}", pid = std::process::id()));
    let (source_dir, source_name_str) = open_parent_dir(worker)?;
    let (destination_dir, destination_name_str) = open_parent_dir(&destination_path)?;
    let source_name = Utf8Path::new(source_name_str);
    let destination_name = Utf8Path::new(destination_name_str);

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

    if WORKER_CACHE.set(destination_path.clone()).is_err() {
        // Another test stored the prepared worker path first.
    }
    Ok(destination_path)
}

fn locate_pg_worker_near_target() -> Option<Utf8PathBuf> {
    let exe_path_os = std::env::current_exe().ok()?;
    let exe_path = utf8_path_from_os(exe_path_os.as_os_str());
    let deps_dir = exe_path.parent()?;
    let target_dir = deps_dir.parent()?;
    let worker_path = target_dir.join("pg_worker");
    worker_path.is_file().then_some(worker_path)
}

fn locate_pg_worker_in_path() -> Option<Utf8PathBuf> {
    let path = std::env::var_os("PATH")?;
    for path_entry in std::env::split_paths(&path) {
        let path_dir = utf8_path_from_os(path_entry.as_os_str());
        let candidate = path_dir.join("pg_worker");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn locate_pg_worker_from_env() -> Option<Utf8PathBuf> {
    let worker_path_os = std::env::var_os("PG_EMBEDDED_WORKER")?;
    let worker_path = utf8_path_from_os(worker_path_os.as_os_str());
    let file_name = worker_path.file_name()?;
    (file_name == "pg_worker").then_some(worker_path)
}

fn utf8_path_from_os(value: &OsStr) -> Utf8PathBuf {
    Utf8PathBuf::from(value.to_string_lossy().into_owned())
}
