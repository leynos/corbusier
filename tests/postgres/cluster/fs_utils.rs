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
