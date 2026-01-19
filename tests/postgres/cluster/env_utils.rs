//! Environment helpers for `PostgreSQL` test clusters.

use super::BoxError;
use super::worker_helpers::{locate_pg_worker_path, prepare_pg_worker};
use pg_embedded_setup_unpriv::{ExecutionPrivileges, detect_execution_privileges};
use std::ffi::OsString;
use std::net::TcpListener;

pub(super) fn env_vars_to_os(
    env_vars: &[(String, Option<String>)],
) -> Vec<(OsString, Option<OsString>)> {
    env_vars
        .iter()
        .map(|(key, value)| (OsString::from(key), value.as_ref().map(OsString::from)))
        .collect()
}

pub(super) fn worker_env_changes() -> Result<Vec<(OsString, Option<OsString>)>, BoxError> {
    let port_override = resolve_pg_port()?;

    let mut changes = Vec::new();
    if let Some(port) = port_override {
        changes.push((OsString::from("PG_PORT"), Some(port)));
    }

    if matches!(detect_execution_privileges(), ExecutionPrivileges::Root)
        && std::env::var_os("PG_EMBEDDED_WORKER").is_none()
    {
        let worker_path = locate_pg_worker_path().ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "PG_EMBEDDED_WORKER is not set and pg_worker binary was not found",
            )) as BoxError
        })?;

        let prepared_worker = prepare_pg_worker(&worker_path)?;
        changes.push((
            OsString::from("PG_EMBEDDED_WORKER"),
            Some(OsString::from(prepared_worker.as_str())),
        ));
    }

    Ok(changes)
}

fn resolve_pg_port() -> Result<Option<OsString>, BoxError> {
    if std::env::var_os("PG_PORT").is_some() {
        return Ok(None);
    }

    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|err| Box::new(err) as BoxError)?;
    let port = listener
        .local_addr()
        .map(|addr| addr.port())
        .map_err(|err| Box::new(err) as BoxError)?;
    drop(listener);

    Ok(Some(OsString::from(port.to_string())))
}
