//! Filesystem helpers for `PostgreSQL` test clusters.

use super::BoxError;
use camino::Utf8Path;
use cap_std::ambient_authority;
use cap_std::fs_utf8::Dir;
use postgresql_embedded::Settings;

pub(super) fn open_ambient_dir(path: &Utf8Path) -> Result<Dir, BoxError> {
    Dir::open_ambient_dir(path, ambient_authority()).map_err(|err| Box::new(err) as BoxError)
}

pub(super) fn open_parent_dir(path: &Utf8Path) -> Result<(Dir, &str), BoxError> {
    let file_name = path.file_name().ok_or_else(|| {
        Box::new(std::io::Error::other("path must include a file name")) as BoxError
    })?;
    let parent = path.parent().unwrap_or_else(|| Utf8Path::new("."));
    let dir = open_ambient_dir(parent)?;
    Ok((dir, file_name))
}

/// Restores `PostgreSQL`'s standard UTC alias when the bundled database omits it.
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

    // The embedded distribution includes UCT, which is the same zero-offset
    // zone, but some releases omit the UTC alias used by Diesel connections.
    dir.copy(Utf8Path::new("UCT"), &dir, Utf8Path::new("UTC"))
        .map_err(|err| Box::new(err) as BoxError)?;
    Ok(())
}

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
mod tests {
    //! Tests for timezone files in the embedded `PostgreSQL` installation.

    use super::{BoxError, ensure_utc_timezone_file, open_ambient_dir};
    use camino::{Utf8Path, Utf8PathBuf};
    use cap_std::fs_utf8::Dir;
    use postgresql_embedded::Settings;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    struct TemporaryInstall {
        root: Dir,
        directory_name: Utf8PathBuf,
        path: Utf8PathBuf,
    }

    impl TemporaryInstall {
        fn new() -> Result<Self, BoxError> {
            let temp_path = Utf8PathBuf::from_path_buf(std::env::temp_dir()).map_err(|path| {
                Box::new(std::io::Error::other(format!(
                    "temporary directory is not UTF-8: {}",
                    path.display()
                ))) as BoxError
            })?;
            let root = open_ambient_dir(&temp_path)?;
            let directory_name = Utf8PathBuf::from(format!(
                "corbusier-pg-timezone-{}-{}",
                std::process::id(),
                NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            root.create_dir(&directory_name)
                .map_err(|err| Box::new(err) as BoxError)?;
            let path = temp_path.join(&directory_name);
            Ok(Self {
                root,
                directory_name,
                path,
            })
        }
    }

    impl Drop for TemporaryInstall {
        fn drop(&mut self) {
            if let Err(error) = self.root.remove_dir_all(&self.directory_name) {
                tracing::warn!(
                    path = %self.path,
                    %error,
                    "failed to remove temporary PostgreSQL installation"
                );
            }
        }
    }

    /// Adds the omitted UTC alias from UCT and preserves an existing UTC file.
    #[test]
    fn missing_utc_alias_is_copied_once() {
        let temporary = TemporaryInstall::new().expect("temporary installation is created");
        let install_dir =
            open_ambient_dir(&temporary.path).expect("installation directory is opened");
        install_dir
            .create_dir_all(Utf8Path::new("share/timezone"))
            .expect("timezone directory is created");
        let timezone_dir = install_dir
            .open_dir(Utf8Path::new("share/timezone"))
            .expect("timezone directory is opened");
        timezone_dir
            .write(Utf8Path::new("UCT"), b"UTC alias")
            .expect("UCT source file is written");
        let settings = Settings {
            installation_dir: temporary.path.clone().into_std_path_buf(),
            ..Settings::new()
        };

        ensure_utc_timezone_file(&settings).expect("missing UTC alias is restored");
        assert_eq!(
            timezone_dir
                .read_to_string(Utf8Path::new("UTC"))
                .expect("restored UTC alias is readable"),
            "UTC alias"
        );
        timezone_dir
            .write(Utf8Path::new("UTC"), b"existing UTC")
            .expect("existing UTC file is written");
        ensure_utc_timezone_file(&settings).expect("existing UTC alias is preserved");
        assert_eq!(
            timezone_dir
                .read_to_string(Utf8Path::new("UTC"))
                .expect("existing UTC file is readable"),
            "existing UTC"
        );
    }
}
