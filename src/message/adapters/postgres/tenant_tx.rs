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
/// **IMPORTANT**: This function does NOT ensure the tenant exists. Callers
/// performing write operations MUST call `ensure_tenant_exists` explicitly
/// before calling this function to guarantee a valid foreign key target exists.
///
/// This separation allows:
/// - Write operations to explicitly bootstrap tenants when needed
/// - Read operations to use this without side effects
/// - Better visibility of which code paths create tenant rows
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
        set_tenant_context(tx, tenant_id)?;
        body(tx).map_err(TxError::Domain)
    })
    .map_err(E::from_tx_error)
}

/// Runs `body` inside a read-only transaction that sets `app.tenant_id`.
///
/// Unlike [`with_tenant_tx`], this configures the database transaction with
/// `SET TRANSACTION READ ONLY` before setting tenant context, making it
/// suitable for read-only operations that:
/// - Should not perform writes
/// - May run under read-only database credentials
/// - Benefit from `PostgreSQL` rejecting accidental writes
///
/// The caller's domain error `E` must implement [`FromTxError`] so that both
/// Diesel errors and domain errors can be propagated.
pub(crate) fn with_tenant_read_tx<T, E, F>(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    body: F,
) -> Result<T, E>
where
    E: FromTxError<E>,
    F: FnOnce(&mut PgConnection) -> Result<T, E>,
{
    conn.transaction::<T, TxError<E>, _>(|tx| {
        diesel::sql_query("SET TRANSACTION READ ONLY").execute(tx)?;
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
pub(crate) fn ensure_tenant_exists(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> diesel::QueryResult<()> {
    bootstrap_tenant_row(conn, tenant_id)?;
    Ok(())
}

/// Inserts a placeholder tenant row if one does not already exist.
///
/// This is the single source of truth for lazy tenant bootstrapping. The
/// slug is formatted as `tenant-{id}` and the display name as
/// `Tenant {id}`. An `ON CONFLICT (id) DO NOTHING` guard makes the
/// operation idempotent.
///
/// Returns the number of rows affected (1 on first insert, 0 on conflict).
///
/// # Examples
///
/// ```ignore
/// bootstrap_tenant_row(&mut conn, tenant_uuid)?;
/// ```
pub(crate) fn bootstrap_tenant_row(
    conn: &mut PgConnection,
    tenant_id: Uuid,
) -> diesel::QueryResult<usize> {
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
    .execute(conn)
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
