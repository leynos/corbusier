//! Worker binary discovery and preparation helpers.

use super::BoxError;
use super::fs_utils::open_parent_dir;
use crate::worker_locator::EnvReader;
use camino::{Utf8Path, Utf8PathBuf};
#[cfg(unix)]
use cap_std::fs::Permissions;
#[cfg(unix)]
use cap_std::fs::PermissionsExt;
use corbusier::worker::shell_escape;
use pg_embedded_setup_unpriv::worker_process_test_api::{
    WorkerOperation, WorkerRequest, WorkerRequestArgs, run as run_worker,
};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static WORKER_CACHE: OnceLock<Mutex<HashMap<Utf8PathBuf, Utf8PathBuf>>> = OnceLock::new();

/// Locates `pg_worker`, reading the caller's variables through `read_env`.
pub(super) fn locate_pg_worker_path_with(read_env: EnvReader<'_>) -> Option<Utf8PathBuf> {
    crate::worker_locator::locate_pg_worker_path_with(read_env)
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

impl super::ManagedCluster {
    /// Executes one worker operation with the bootstrap environment and timeout.
    ///
    /// Lifecycle methods retain setup/start ordering; a `Setup` request here only
    /// prepares the installation and does not implicitly start the server.
    pub(super) fn run_worker_operation(
        &self,
        operation: WorkerOperation,
        timeout: Duration,
    ) -> Result<(), BoxError> {
        let worker = self.bootstrap.worker_binary.as_ref().ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "PG_EMBEDDED_WORKER is not set for worker operation",
            )) as BoxError
        })?;
        let args = WorkerRequestArgs {
            worker: worker.as_path(),
            settings: &self.bootstrap.settings,
            env_vars: &self.env_vars,
            operation,
            timeout,
        };
        run_worker(&WorkerRequest::new(args)).map_err(|err| Box::new(err) as BoxError)?;
        Ok(())
    }
}

fn write_worker_wrapper(
    destination_dir: &cap_std::fs_utf8::Dir,
    destination_name: &Utf8Path,
    worker_binary_path: &Utf8Path,
) -> Result<(), BoxError> {
    let mut file = destination_dir
        .create(destination_name)
        .map_err(|err| Box::new(err) as BoxError)?;
    let worker_path = shell_escape(worker_binary_path.as_str());
    let script = format!(
        concat!(
            "#!/bin/sh\n",
            "if [ \"$(id -u)\" -eq 0 ]; then\n",
            "  if command -v runuser >/dev/null 2>&1; then\n",
            "    exec runuser -u nobody -- {worker} \"$@\"\n",
            "  fi\n",
            "  exec su -s /bin/sh nobody -c 'exec \"$0\" \"$@\"' {worker} \"$@\"\n",
            "fi\n",
            "exec {worker} \"$@\"\n",
        ),
        worker = worker_path
    );
    file.write_all(script.as_bytes())
        .map_err(|err| Box::new(err) as BoxError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Tests for `pg_worker` discovery and preparation helpers.

    use super::{locate_pg_worker_path_with, prepare_pg_worker};
    use crate::worker_locator::{locate_pg_worker_in_path, locate_pg_worker_path_from};
    use camino::{Utf8Path, Utf8PathBuf};
    use cap_std::ambient_authority;
    #[cfg(unix)]
    use cap_std::fs::Permissions;
    #[cfg(unix)]
    use cap_std::fs::PermissionsExt;
    use cap_std::fs_utf8::Dir;
    use std::ffi::OsString;
    use std::io::Write;

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

    #[cfg(unix)]
    fn open_parent_dir(path: &Utf8Path) -> Result<(Dir, String), std::io::Error> {
        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("path has no parent: {path}"),
            )
        })?;
        let file_name = path
            .file_name()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("path has no file name: {path}"),
                )
            })?
            .to_owned();
        let dir = Dir::open_ambient_dir(parent, ambient_authority())?;
        Ok((dir, file_name))
    }

    #[cfg(unix)]
    fn set_file_mode(path: &Utf8Path, mode: u32) -> Result<(), std::io::Error> {
        let (dir, file_name) = open_parent_dir(path)?;
        dir.set_permissions(Utf8Path::new(&file_name), Permissions::from_mode(mode))
    }

    #[cfg(unix)]
    fn file_mode(path: &Utf8Path) -> Result<u32, std::io::Error> {
        let (dir, file_name) = open_parent_dir(path)?;
        let metadata = dir.metadata(Utf8Path::new(&file_name))?;
        Ok(metadata.permissions().mode())
    }

    #[test]
    fn env_var_takes_precedence_over_other_discovery() {
        let env_dir = create_temp_dir("pg_worker_env").expect("failed to create env worker dir");
        let env_worker =
            write_worker_script(&env_dir, "pg_worker").expect("failed to write env worker");
        let path_dir = create_temp_dir("pg_worker_path").expect("failed to create PATH worker dir");
        let _path_worker =
            write_worker_script(&path_dir, "pg_worker").expect("failed to write PATH worker");

        let env = [
            (
                "CARGO_BIN_EXE_pg_worker",
                OsString::from(env_worker.as_str()),
            ),
            ("PATH", OsString::from(path_dir.as_str())),
        ];

        let located = locate_pg_worker_path_with(&|key| lookup(&env, key))
            .expect("expected worker path from CARGO_BIN_EXE_pg_worker");
        assert_eq!(
            located, env_worker,
            "CARGO_BIN_EXE_pg_worker should take precedence over PATH discovery",
        );
    }

    /// The value `env` holds for `key`; the injected environment of these tests.
    fn lookup(env: &[(&str, OsString)], key: &str) -> Option<OsString> {
        env.iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.clone())
    }

    #[test]
    fn falls_back_to_path_when_env_unset() {
        let path_dir =
            create_temp_dir("pg_worker_fallback").expect("failed to create PATH worker dir");
        let _path_worker =
            write_worker_script(&path_dir, "pg_worker").expect("failed to write PATH worker");

        let env = [("PATH", OsString::from(path_dir.as_str()))];

        // The full discovery checks the test executable's own directory before
        // `PATH`, which an injected environment cannot hide, so the `PATH` step
        // is exercised on its own to prove it reads the injected value.
        let located = locate_pg_worker_in_path(&|key| lookup(&env, key))
            .expect("expected worker path from the injected PATH");
        assert_eq!(
            located,
            path_dir.join("pg_worker"),
            "the PATH step should find the fixture worker in the injected PATH"
        );
        // Full discovery with the executable placed where no worker sits beside
        // it (`no-worker/pg_worker` does not exist; the fixture is one level
        // up), so every earlier step finds nothing and the injected `PATH`
        // decides.
        let lonely_exe = std::path::PathBuf::from(path_dir.as_str())
            .join("no-worker")
            .join("deps")
            .join("test_binary");
        let full = locate_pg_worker_path_from(&|key| lookup(&env, key), Some(&lonely_exe));
        assert_eq!(
            full,
            Some(path_dir.join("pg_worker")),
            "full discovery should reach the injected PATH and find the fixture worker"
        );
    }

    #[test]
    fn prepare_pg_worker_is_idempotent_and_sets_permissions() {
        let source_dir =
            create_temp_dir("pg_worker_source").expect("failed to create worker source dir");
        let worker_path =
            write_worker_script(&source_dir, "pg_worker").expect("failed to write worker script");

        #[cfg(unix)]
        {
            set_file_mode(&worker_path, 0o600).expect("failed to set worker permissions");
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
            let mode = file_mode(&prepared).expect("failed to read prepared metadata");
            assert!(
                mode & 0o111 != 0,
                "expected prepared worker to be executable, mode {mode:o}",
            );
        }
    }
}
