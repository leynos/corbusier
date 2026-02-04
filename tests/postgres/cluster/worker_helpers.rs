//! Worker binary discovery and preparation helpers.

use super::BoxError;
use super::fs_utils::open_parent_dir;
use camino::{Utf8Path, Utf8PathBuf};
#[cfg(unix)]
use cap_std::fs::Permissions;
#[cfg(unix)]
use cap_std::fs::PermissionsExt;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::{Mutex, OnceLock};

static WORKER_CACHE: OnceLock<Mutex<HashMap<Utf8PathBuf, Utf8PathBuf>>> = OnceLock::new();

pub(super) fn locate_pg_worker_path() -> Option<Utf8PathBuf> {
    crate::test_helpers::locate_pg_worker_path()
}

pub(super) fn prepare_pg_worker(worker: &Utf8Path) -> Result<Utf8PathBuf, BoxError> {
    let key = worker.to_path_buf();
    let cache = WORKER_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let cached_path = cache
        .lock()
        .map_err(|err| {
            Box::new(std::io::Error::other(format!(
                "worker cache lock poisoned: {err}"
            ))) as BoxError
        })?
        .get(&key)
        .cloned();
    if let Some(path) = cached_path {
        return Ok(path);
    }

    let temp_dir = Utf8PathBuf::try_from(std::env::temp_dir()).map_err(|e| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("temp directory path is not valid UTF-8: {e}"),
        )) as BoxError
    })?;
    let mut hasher = DefaultHasher::new();
    key.as_str().hash(&mut hasher);
    let hash = hasher.finish();
    let destination_path = temp_dir.join(format!(
        "pg_worker_{pid}_{hash:x}",
        pid = std::process::id()
    ));
    let worker_binary_path = destination_path.with_extension("bin");
    let (source_dir, source_name_str) = open_parent_dir(worker)?;
    let (destination_dir, destination_name_str) = open_parent_dir(&destination_path)?;
    let (worker_dir, worker_name_str) = open_parent_dir(&worker_binary_path)?;
    let source_name = Utf8Path::new(source_name_str);
    let destination_name = Utf8Path::new(destination_name_str);
    let worker_name = Utf8Path::new(worker_name_str);

    match destination_dir.remove_file(destination_name) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(Box::new(err) as BoxError),
    }
    match worker_dir.remove_file(worker_name) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(Box::new(err) as BoxError),
    }

    source_dir
        .copy(source_name, &worker_dir, worker_name)
        .map_err(|err| Box::new(err) as BoxError)?;
    write_worker_wrapper(&destination_dir, destination_name, &worker_binary_path)?;

    #[cfg(unix)]
    {
        destination_dir
            .set_permissions(destination_name, Permissions::from_mode(0o755))
            .map_err(|err| Box::new(err) as BoxError)?;
        worker_dir
            .set_permissions(worker_name, Permissions::from_mode(0o755))
            .map_err(|err| Box::new(err) as BoxError)?;
    }

    let mut locked_cache = cache.lock().map_err(|err| {
        Box::new(std::io::Error::other(format!(
            "worker cache lock poisoned: {err}"
        ))) as BoxError
    })?;
    let result_path = locked_cache
        .entry(key)
        .or_insert_with(|| destination_path.clone());
    Ok(result_path.clone())
}

fn write_worker_wrapper(
    destination_dir: &cap_std::fs_utf8::Dir,
    destination_name: &Utf8Path,
    worker_binary_path: &Utf8Path,
) -> Result<(), BoxError> {
    let mut file = destination_dir
        .create(destination_name)
        .map_err(|err| Box::new(err) as BoxError)?;
    let script = format!(
        concat!(
            "#!/bin/sh\n",
            "if [ \"$(id -u)\" -eq 0 ]; then\n",
            "  exec /usr/sbin/runuser -u nobody -- {worker} \"$@\"\n",
            "fi\n",
            "exec {worker} \"$@\"\n",
        ),
        worker = worker_binary_path.as_str()
    );
    file.write_all(script.as_bytes())
        .map_err(|err| Box::new(err) as BoxError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests for `pg_worker` discovery and preparation helpers.

    use super::{locate_pg_worker_path, prepare_pg_worker};
    use crate::test_helpers::EnvVarGuard;
    use camino::{Utf8Path, Utf8PathBuf};
    use cap_std::ambient_authority;
    use cap_std::fs_utf8::Dir;
    use std::ffi::OsString;
    use std::io::Write;

    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt as StdPermissionsExt;

    fn create_temp_dir(prefix: &str) -> Result<Utf8PathBuf, std::io::Error> {
        let base = Utf8PathBuf::try_from(std::env::temp_dir()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("temp directory path is not valid UTF-8: {e}"),
            )
        })?;
        let dir_name = format!("{prefix}_{}", uuid::Uuid::new_v4());
        let base_dir = Dir::open_ambient_dir(&base, ambient_authority())?;
        base_dir.create_dir(&dir_name)?;
        Ok(base.join(dir_name))
    }

    fn write_worker_script(dir: &Utf8Path, name: &str) -> Result<Utf8PathBuf, std::io::Error> {
        let dir_handle = Dir::open_ambient_dir(dir, ambient_authority())?;
        let mut file = dir_handle.create(name)?;
        file.write_all(b"#!/bin/sh\nexit 0\n")?;
        Ok(dir.join(name))
    }

    #[test]
    fn env_var_takes_precedence_over_other_discovery() {
        let env_dir = create_temp_dir("pg_worker_env").expect("failed to create env worker dir");
        let env_worker =
            write_worker_script(&env_dir, "pg_worker").expect("failed to write env worker");
        let path_dir = create_temp_dir("pg_worker_path").expect("failed to create PATH worker dir");
        let _path_worker =
            write_worker_script(&path_dir, "pg_worker").expect("failed to write PATH worker");

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

        let located =
            locate_pg_worker_path().expect("expected worker path from CARGO_BIN_EXE_pg_worker");
        assert_eq!(
            located, env_worker,
            "CARGO_BIN_EXE_pg_worker should take precedence over PATH discovery",
        );
        drop(guard);
    }

    #[test]
    fn falls_back_to_path_when_env_unset() {
        let path_dir =
            create_temp_dir("pg_worker_fallback").expect("failed to create PATH worker dir");
        let _path_worker =
            write_worker_script(&path_dir, "pg_worker").expect("failed to write PATH worker");

        let guard = EnvVarGuard::set_many(&[
            (OsString::from("CARGO_BIN_EXE_pg_worker"), None),
            (OsString::from("PG_EMBEDDED_WORKER"), None),
            (
                OsString::from("PATH"),
                Some(OsString::from(path_dir.as_str())),
            ),
        ]);

        let located =
            locate_pg_worker_path().expect("expected worker path from fallback discovery");
        assert!(
            located.is_file(),
            "expected fallback to locate a worker binary, got {located}"
        );
        drop(guard);
    }

    #[test]
    fn prepare_pg_worker_is_idempotent_and_sets_permissions() {
        let source_dir =
            create_temp_dir("pg_worker_source").expect("failed to create worker source dir");
        let worker_path =
            write_worker_script(&source_dir, "pg_worker").expect("failed to write worker script");

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(worker_path.as_std_path())
                .expect("failed to read worker permissions")
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(worker_path.as_std_path(), perms)
                .expect("failed to set worker permissions");
        }

        let prepared = prepare_pg_worker(&worker_path).expect("failed to prepare worker binary");
        let prepared_second =
            prepare_pg_worker(&worker_path).expect("failed to prepare worker binary");
        assert_eq!(
            prepared, prepared_second,
            "prepare_pg_worker should reuse the cached worker path",
        );

        #[cfg(unix)]
        {
            let meta =
                fs::metadata(prepared.as_std_path()).expect("failed to read prepared metadata");
            let mode = meta.permissions().mode();
            assert!(
                mode & 0o111 != 0,
                "expected prepared worker to be executable, mode {mode:o}",
            );
        }
    }
}
