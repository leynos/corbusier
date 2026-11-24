//! Worker binary discovery helpers for integration tests.

use camino::Utf8PathBuf;
use std::env;
use std::ffi::{OsStr, OsString};

/// Reads one environment variable by name; `None` when it is unset.
///
/// Discovery takes its reader as a parameter so tests can inject an
/// environment instead of changing the process's own.
pub type EnvReader<'a> = &'a dyn Fn(&str) -> Option<OsString>;

/// Reads the process environment.
#[must_use]
pub fn process_env(key: &str) -> Option<OsString> {
    env::var_os(key)
}

/// Returns the platform-appropriate executable name for `pg_worker`.
const fn pg_worker_binary_name() -> &'static str {
    if cfg!(windows) {
        "pg_worker.exe"
    } else {
        "pg_worker"
    }
}

/// Locates the `pg_worker` binary for integration tests, reading variables
/// through `read_env`.
#[must_use]
pub fn locate_pg_worker_path_with(read_env: EnvReader<'_>) -> Option<Utf8PathBuf> {
    read_env("CARGO_BIN_EXE_pg_worker")
        .and_then(|path| utf8_path_from_os(path.as_os_str()))
        .or_else(|| locate_pg_worker_from_env(read_env))
        .or_else(|| locate_pg_worker_in_cargo_bin(read_env))
        .or_else(locate_pg_worker_near_target)
        .or_else(|| locate_pg_worker_in_path(read_env))
}

fn locate_pg_worker_in_cargo_bin(read_env: EnvReader<'_>) -> Option<Utf8PathBuf> {
    let home = read_env("HOME").or_else(|| read_env("USERPROFILE"))?;
    let home_path = utf8_path_from_os(home.as_os_str())?;
    let worker_path = home_path
        .join(".cargo")
        .join("bin")
        .join(pg_worker_binary_name());
    worker_path.is_file().then_some(worker_path)
}

fn locate_pg_worker_near_target() -> Option<Utf8PathBuf> {
    let exe_path = env::current_exe().ok()?;
    let exe_path_utf8 = utf8_path_from_os(exe_path.as_os_str())?;
    let deps_dir = exe_path_utf8.parent()?;
    let target_dir = deps_dir.parent()?;
    let worker_path = target_dir.join(pg_worker_binary_name());
    worker_path.is_file().then_some(worker_path)
}

/// Searches the injected `PATH` for `pg_worker`, the last step of discovery.
///
/// Public so a test can reach this step alone: the steps before it consult the
/// test executable's own directory, which an injected environment cannot hide.
#[must_use]
pub fn locate_pg_worker_in_path(read_env: EnvReader<'_>) -> Option<Utf8PathBuf> {
    let path = read_env("PATH")?;
    for path_entry in env::split_paths(&path) {
        let Some(path_dir) = utf8_path_from_os(path_entry.as_os_str()) else {
            continue;
        };
        let candidate = path_dir.join(pg_worker_binary_name());
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn locate_pg_worker_from_env(read_env: EnvReader<'_>) -> Option<Utf8PathBuf> {
    let worker_path = read_env("PG_EMBEDDED_WORKER")?;
    let worker_path_utf8 = utf8_path_from_os(worker_path.as_os_str())?;
    let file_stem = worker_path_utf8.file_stem()?;
    if file_stem == "pg_worker" && worker_path_utf8.is_file() {
        Some(worker_path_utf8)
    } else {
        None
    }
}

fn utf8_path_from_os(value: &OsStr) -> Option<Utf8PathBuf> {
    Some(Utf8PathBuf::from(value.to_os_string().into_string().ok()?))
}
