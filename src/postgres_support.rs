//! Shared `PostgreSQL` support helpers for tenant-scoped adapters.
//!
//! This facade keeps bounded contexts from depending directly on another
//! adapter module's internal layout while still reusing the canonical tenant
//! transaction and blocking-execution helpers.

use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PoolError, PooledConnection};

pub(crate) use crate::message::adapters::postgres::tenant_tx::{
    FromTxError, TxError, ensure_tenant_exists, with_tenant_read_tx, with_tenant_tx,
};

/// Shared `PostgreSQL` connection pool type.
pub type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Shared pooled connection type for synchronous Diesel work.
pub(crate) type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;

/// Runs a blocking task and maps join errors into the caller's error type.
pub(crate) async fn run_blocking_with<F, T, E, M>(f: F, map_err: M) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
    M: FnOnce(tokio::task::JoinError) -> E,
{
    tokio::task::spawn_blocking(f).await.map_err(map_err)?
}

/// Obtains a connection from the pool with a caller-provided error mapper.
pub(crate) fn get_conn_with<E, M>(pool: &PgPool, map_err: M) -> Result<PooledConn, E>
where
    M: FnOnce(PoolError) -> E,
{
    pool.get().map_err(map_err)
}
