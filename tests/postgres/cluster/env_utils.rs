//! Environment helpers for `PostgreSQL` test clusters.

use super::BoxError;
use super::worker_helpers::{locate_pg_worker_path, prepare_pg_worker};
use crate::test_helpers::EnvVarGuard;
use camino::{Utf8Path, Utf8PathBuf};
use nix::unistd::{Uid, User, initgroups, setgid, setuid};
use pg_embedded_setup_unpriv::{ExecutionPrivileges, detect_execution_privileges};
use std::ffi::{CString, OsString};
use std::net::TcpListener;

type WorkerEnvChanges = (Vec<(OsString, Option<OsString>)>, Option<TcpListener>);

pub(super) fn env_vars_to_os(
    env_vars: &[(String, Option<String>)],
) -> Vec<(OsString, Option<OsString>)> {
    env_vars
        .iter()
        .map(|(key, value)| (OsString::from(key), value.as_ref().map(OsString::from)))
        .collect()
}

pub(super) fn worker_env_changes() -> Result<WorkerEnvChanges, BoxError> {
    let (mut changes, port_guard) = worker_env_changes_impl(
        detect_execution_privileges,
        locate_pg_worker_path,
        prepare_pg_worker,
    )?;
    let root = shared_directory_root(&std::env::temp_dir(), Uid::effective().as_raw());
    changes.extend(shared_directory_changes(&root, |key| std::env::var_os(key)));
    Ok((changes, port_guard))
}

/// Returns the root shared by every test process of one user.
///
/// The user ID is part of the name so two users on one host never share, and
/// never fight over the ownership of, a cluster directory.
fn shared_directory_root(temp_dir: &std::path::Path, uid: u32) -> std::path::PathBuf {
    temp_dir.join(format!("corbusier-pg-embed-{uid}"))
}

/// Pins one install and data directory under `root` for every test process.
///
/// This suite shares a single cluster across processes: the first process runs
/// `initdb`, and later ones find the initialized directory and only start the
/// server. `pg-embed-setup-unpriv` 0.6 gives each process a directory of its own
/// unless `PG_DATA_DIR` is set, which would make every test process run
/// `initdb` and leave a server behind for the next sweep. Setting the variables
/// restores the sharing. A data directory the caller already set wins and
/// nothing is pinned; a runtime directory the caller set is kept while the data
/// directory is pinned. The caller supplies `root` and the environment lookup so
/// the function touches neither the operating system nor the process
/// environment.
fn shared_directory_changes(
    root: &std::path::Path,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> Vec<(OsString, Option<OsString>)> {
    if lookup("PG_DATA_DIR").is_some() {
        return Vec::new();
    }
    let mut changes = vec![(
        OsString::from("PG_DATA_DIR"),
        Some(root.join("data").into_os_string()),
    )];
    if lookup("PG_RUNTIME_DIR").is_none() {
        changes.push((
            OsString::from("PG_RUNTIME_DIR"),
            Some(root.join("install").into_os_string()),
        ));
    }
    changes
}

pub(super) fn drop_privileges_if_root(
    username: &str,
    env_vars: &[(OsString, Option<OsString>)],
) -> Result<Option<EnvVarGuard>, BoxError> {
    if !Uid::effective().is_root() {
        return Ok(None);
    }

    let user = User::from_name(username)
        .map_err(|err| Box::new(err) as BoxError)?
        .ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("user '{username}' not found"),
            )) as BoxError
        })?;

    let account_name = user.name.clone();
    let user_home = user.dir.clone().into_os_string();
    let mut changes = Vec::with_capacity(env_vars.len() + 3);
    changes.extend(env_vars.iter().cloned());
    changes.push((OsString::from("HOME"), Some(user_home)));
    changes.push((
        OsString::from("USER"),
        Some(OsString::from(account_name.clone())),
    ));
    changes.push((
        OsString::from("LOGNAME"),
        Some(OsString::from(account_name.clone())),
    ));

    let env_guard = EnvVarGuard::set_many(&changes);

    let user_cstr = CString::new(user.name.clone()).map_err(|err| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid user name for initgroups: {err}"),
        )) as BoxError
    })?;
    initgroups(&user_cstr, user.gid).map_err(|err| Box::new(err) as BoxError)?;
    setgid(user.gid).map_err(|err| Box::new(err) as BoxError)?;
    setuid(user.uid).map_err(|err| Box::new(err) as BoxError)?;

    Ok(Some(env_guard))
}

fn worker_env_changes_impl<D, L, P>(
    detect_privileges: D,
    locate_worker: L,
    prepare_worker: P,
) -> Result<WorkerEnvChanges, BoxError>
where
    D: Fn() -> ExecutionPrivileges,
    L: Fn() -> Option<Utf8PathBuf>,
    P: Fn(&Utf8Path) -> Result<Utf8PathBuf, BoxError>,
{
    let port_override = resolve_pg_port()?;

    let mut changes = Vec::new();
    let port_guard = if let Some((port, listener)) = port_override {
        changes.push((OsString::from("PG_PORT"), Some(port)));
        Some(listener)
    } else {
        None
    };

    if !matches!(detect_privileges(), ExecutionPrivileges::Root) {
        return Ok((changes, port_guard));
    }

    if std::env::var_os("PG_EMBEDDED_WORKER").is_some() {
        return Ok((changes, port_guard));
    }

    let worker_path = locate_worker().ok_or_else(|| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "PG_EMBEDDED_WORKER is not set and pg_worker binary was not found",
        )) as BoxError
    })?;
    let prepared_path = prepare_worker(&worker_path)?;
    changes.push((
        OsString::from("PG_EMBEDDED_WORKER"),
        Some(OsString::from(prepared_path.as_str())),
    ));

    Ok((changes, port_guard))
}

fn resolve_pg_port() -> Result<Option<(OsString, TcpListener)>, BoxError> {
    if std::env::var_os("PG_PORT").is_some() {
        return Ok(None);
    }

    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|err| Box::new(err) as BoxError)?;
    let port = listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|err| Box::new(err) as BoxError)?;

    Ok(Some((OsString::from(port.to_string()), listener)))
}

#[cfg(test)]
mod tests {
    //! Tests for worker environment changes and PG port handling.

    use super::worker_env_changes_impl;
    use crate::test_helpers::EnvVarGuard;
    use camino::Utf8PathBuf;
    use pg_embedded_setup_unpriv::ExecutionPrivileges;
    use std::ffi::OsString;
    use std::io;

    fn dummy_worker_path() -> Utf8PathBuf {
        let base = Utf8PathBuf::try_from(std::env::temp_dir())
            .expect("temp directory path is not valid UTF-8");
        base.join(format!("pg_worker_test_{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn preserves_existing_pg_port() {
        let guard = EnvVarGuard::set_many(&[
            (OsString::from("PG_PORT"), Some(OsString::from("54321"))),
            (OsString::from("PG_EMBEDDED_WORKER"), None),
        ]);

        let (changes, _listener) = worker_env_changes_impl(
            || ExecutionPrivileges::Unprivileged,
            || None,
            |_| Ok(dummy_worker_path()),
        )
        .expect("worker env changes failed");

        let pg_port = OsString::from("PG_PORT");
        let has_pg_port = changes.iter().any(|(key, _)| key == &pg_port);
        assert!(
            !has_pg_port,
            "expected no PG_PORT override when PG_PORT is already set",
        );
        drop(guard);
    }

    #[test]
    fn non_root_does_not_emit_pg_embedded_worker() {
        let guard = EnvVarGuard::set_many(&[
            (OsString::from("PG_PORT"), None),
            (OsString::from("PG_EMBEDDED_WORKER"), None),
        ]);

        let (changes, _listener) = worker_env_changes_impl(
            || ExecutionPrivileges::Unprivileged,
            || Some(dummy_worker_path()),
            |_| Ok(dummy_worker_path()),
        )
        .expect("worker env changes failed");

        let worker_key = OsString::from("PG_EMBEDDED_WORKER");
        let has_worker = changes.iter().any(|(key, _)| key == &worker_key);
        assert!(
            !has_worker,
            "expected no PG_EMBEDDED_WORKER change for non-root execution",
        );
        drop(guard);
    }

    #[test]
    fn root_without_worker_yields_not_found() {
        let guard = EnvVarGuard::set_many(&[
            (OsString::from("PG_PORT"), None),
            (OsString::from("PG_EMBEDDED_WORKER"), None),
        ]);

        let result = worker_env_changes_impl(
            || ExecutionPrivileges::Root,
            || None,
            |_| Ok(dummy_worker_path()),
        );

        let error = result
            .map(|(changes, _listener)| changes)
            .expect_err("expected worker lookup failure for root execution");
        let io_err = error
            .downcast_ref::<io::Error>()
            .expect("expected io::Error for missing pg_worker");

        assert_eq!(io_err.kind(), io::ErrorKind::NotFound);
        drop(guard);
    }

    #[test]
    fn root_sets_pg_embedded_worker_when_discoverable() {
        let guard = EnvVarGuard::set_many(&[
            (OsString::from("PG_PORT"), None),
            (OsString::from("PG_EMBEDDED_WORKER"), None),
        ]);
        let expected_path = dummy_worker_path();
        let expected_os = OsString::from(expected_path.as_str());

        let (changes, _listener) = worker_env_changes_impl(
            || ExecutionPrivileges::Root,
            || Some(expected_path.clone()),
            |_| Ok(expected_path.clone()),
        )
        .expect("worker env changes failed");

        let worker_key = OsString::from("PG_EMBEDDED_WORKER");
        let worker_value = changes
            .iter()
            .find(|(key, _)| key == &worker_key)
            .and_then(|(_, value)| value.clone())
            .expect("expected PG_EMBEDDED_WORKER to be set");

        assert_eq!(worker_value, expected_os);
        drop(guard);
    }
}

#[cfg(test)]
mod shared_directory_tests {
    //! Tests for the pinned shared install and data directories.

    use super::{shared_directory_changes, shared_directory_root};
    use rstest::rstest;
    use std::ffi::OsString;
    use std::path::Path;

    fn pinned(values: &[(OsString, Option<OsString>)]) -> Vec<(String, String)> {
        values
            .iter()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value
                        .as_ref()
                        .map(|v| v.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                )
            })
            .collect()
    }

    /// The root is one directory per user under the given temporary directory.
    #[rstest]
    #[case(1000, "/tmp/corbusier-pg-embed-1000")]
    #[case(0, "/tmp/corbusier-pg-embed-0")]
    fn the_root_is_named_for_the_user(#[case] uid: u32, #[case] expected: &str) {
        assert_eq!(
            shared_directory_root(Path::new("/tmp"), uid),
            Path::new(expected)
        );
    }

    /// Which variables the caller already set decides what is pinned, and the
    /// pinned values are the `data` and `install` children of one shared root.
    #[rstest]
    #[case::none_set(&[], &[("PG_DATA_DIR", "/r/data"), ("PG_RUNTIME_DIR", "/r/install")])]
    #[case::data_dir_set(&["PG_DATA_DIR"], &[])]
    #[case::runtime_dir_set(&["PG_RUNTIME_DIR"], &[("PG_DATA_DIR", "/r/data")])]
    #[case::both_set(&["PG_DATA_DIR", "PG_RUNTIME_DIR"], &[])]
    fn the_caller_s_variables_win(#[case] preset: &[&str], #[case] expected: &[(&str, &str)]) {
        let changes = shared_directory_changes(Path::new("/r"), |key| {
            preset.contains(&key).then(|| OsString::from("/elsewhere"))
        });
        let want: Vec<(String, String)> = expected
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        assert_eq!(pinned(&changes), want);
    }
}
