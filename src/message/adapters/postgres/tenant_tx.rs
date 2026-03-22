//! Shared transaction helper for tenant-scoped `PostgreSQL` operations.
//!
//! Provides [`with_tenant_tx`] which wraps a closure in a database transaction
//! that first sets the `app.tenant_id` session variable, preparing the
//! connection for Row-Level Security (RLS) policies.
//!
//! The adapter-local [`TxError`] wrapper satisfies Diesel's
//! `From<diesel::result::Error>` bound on [`PgConnection::transaction`] without
//! leaking Diesel types into the port layer.

use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

/// Adapter-local wrapper that satisfies Diesel's `From<diesel::result::Error>`
/// bound on [`PgConnection::transaction`] without leaking Diesel types into
/// the port layer.
///
/// `E` is the caller's domain error type (e.g. `SessionError`, `HandoffError`).
pub(crate) enum TxError<E> {
    Domain(E),
    Diesel(diesel::result::Error),
}

impl<E> From<diesel::result::Error> for TxError<E> {
    fn from(err: diesel::result::Error) -> Self {
        Self::Diesel(err)
    }
}

/// Converts a [`TxError`] back into the caller's domain error.
///
/// Implementors map `TxError::Diesel` into their own persistence error variant
/// and unwrap `TxError::Domain` transparently.
pub(crate) trait FromTxError<E> {
    fn from_tx_error(err: TxError<E>) -> E;
}

/// Runs `body` inside a transaction that first sets `app.tenant_id`.
///
/// The caller's domain error `E` must implement [`FromTxError`] so that both
/// Diesel errors and domain errors can be propagated.
pub(crate) fn with_tenant_tx<T, E, F>(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    body: F,
) -> Result<T, E>
where
    E: FromTxError<E>,
    F: FnOnce(&mut PgConnection) -> Result<T, E>,
{
    conn.transaction::<T, TxError<E>, _>(|tx| {
        ensure_tenant_exists(tx, tenant_id)?;
        set_tenant_context(tx, tenant_id)?;
        body(tx).map_err(TxError::Domain)
    })
    .map_err(E::from_tx_error)
}

/// Ensures a lightweight tenant row exists for the given identifier.
///
/// Tenant lifecycle management is delivered separately from this milestone,
/// but adapter writes already require a valid foreign key target. We
/// therefore provision a stable placeholder row on first use so persistence
/// remains compatible with the existing `RequestContext` contract.
pub(crate) fn ensure_tenant_exists<E>(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<(), TxError<E>> {
    let tenant_slug = format!("tenant-{tenant_id}");
    let tenant_name = format!("Tenant {tenant_id}");

    diesel::sql_query(concat!(
        "INSERT INTO tenants (id, slug, name, status, created_at, updated_at) ",
        "VALUES ($1, $2, $3, 'active', NOW(), NOW()) ",
        "ON CONFLICT (id) DO NOTHING",
    ))
    .bind::<diesel::sql_types::Uuid, _>(tenant_id)
    .bind::<diesel::sql_types::Text, _>(tenant_slug)
    .bind::<diesel::sql_types::Text, _>(tenant_name)
    .execute(conn)?;

    Ok(())
}

/// Sets the `PostgreSQL` session variable `app.tenant_id` for the current
/// transaction.
///
/// This prepares the connection for Row-Level Security (RLS) policies.
/// `SET LOCAL` scopes the variable to the enclosing transaction, so each
/// request gets an isolated tenant context.
///
/// # Security
///
/// UUID values are formatted using their canonical hyphenated representation
/// which contains only hexadecimal digits and hyphens, making SQL injection
/// impossible.
pub(crate) fn set_tenant_context<E>(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> Result<(), TxError<E>> {
    diesel::sql_query(format!("SET LOCAL app.tenant_id = '{tenant_id}'")).execute(conn)?;
    Ok(())
}
