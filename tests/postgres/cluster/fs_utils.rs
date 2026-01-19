//! Filesystem helpers for PostgreSQL test clusters.

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
    let password_path = settings.password_file.to_string_lossy();
    let password_path = Utf8Path::new(password_path.as_ref());
    let (dir, file_name) = open_parent_dir(password_path)?;
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
    let data_dir = settings.data_dir.to_string_lossy();
    let data_dir = Utf8Path::new(data_dir.as_ref());
    let data_dir = open_ambient_dir(data_dir)?;
    let contents = match data_dir.read_to_string("postmaster.pid") {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Box::new(err) as BoxError),
    };

    let port_line = contents.lines().nth(3).map(str::trim);
    let Some(port_value) = port_line else {
        return Ok(());
    };
    let Ok(port) = port_value.parse::<u16>() else {
        return Ok(());
    };
    settings.port = port;
    Ok(())
}
