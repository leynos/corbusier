//! Worker binary discovery helpers for integration tests.

use camino::Utf8PathBuf;
use std::env;
use std::ffi::OsStr;

/// Locates the `pg_worker` binary for integration tests.
pub fn locate_pg_worker_path() -> Option<Utf8PathBuf> {
    env::var_os("CARGO_BIN_EXE_pg_worker")
        .and_then(|path| utf8_path_from_os(path.as_os_str()))
        .or_else(locate_pg_worker_in_cargo_bin)
        .or_else(locate_pg_worker_near_target)
        .or_else(locate_pg_worker_in_path)
        .or_else(locate_pg_worker_from_env)
}

fn locate_pg_worker_in_cargo_bin() -> Option<Utf8PathBuf> {
    let home = env::var_os("HOME")?;
    let home_path = utf8_path_from_os(home.as_os_str())?;
    let worker_path = home_path.join(".cargo").join("bin").join("pg_worker");
    worker_path.is_file().then_some(worker_path)
}

fn locate_pg_worker_near_target() -> Option<Utf8PathBuf> {
    let exe_path = env::current_exe().ok()?;
    let exe_path_utf8 = utf8_path_from_os(exe_path.as_os_str())?;
    let deps_dir = exe_path_utf8.parent()?;
    let target_dir = deps_dir.parent()?;
    let worker_path = target_dir.join("pg_worker");
    worker_path.is_file().then_some(worker_path)
}

fn locate_pg_worker_in_path() -> Option<Utf8PathBuf> {
    let path = env::var_os("PATH")?;
    for path_entry in env::split_paths(&path) {
        let Some(path_dir) = utf8_path_from_os(path_entry.as_os_str()) else {
            continue;
        };
        let candidate = path_dir.join("pg_worker");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn locate_pg_worker_from_env() -> Option<Utf8PathBuf> {
    let worker_path = env::var_os("PG_EMBEDDED_WORKER")?;
    let worker_path_utf8 = utf8_path_from_os(worker_path.as_os_str())?;
    let file_name = worker_path_utf8.file_name()?;
    (file_name == "pg_worker").then_some(worker_path_utf8)
}

fn utf8_path_from_os(value: &OsStr) -> Option<Utf8PathBuf> {
    Some(Utf8PathBuf::from(value.to_os_string().into_string().ok()?))
}
