//! Blocking operation helpers for `PostgreSQL` repository.
//!
//! Provides utilities for offloading synchronous Diesel operations to a
//! dedicated thread pool, avoiding blocking the async executor.

use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PoolError, PooledConnection};

use crate::message::{error::RepositoryError, ports::repository::RepositoryResult};

/// `PostgreSQL` connection pool type.
pub type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Pooled connection type for internal use.
pub(super) type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;

/// Runs a blocking database operation on a dedicated thread pool.
///
/// Wraps the closure in [`tokio::task::spawn_blocking`] to prevent
/// blocking the async executor's worker threads.
pub(super) async fn run_blocking<F, T>(f: F) -> RepositoryResult<T>
where
    F: FnOnce() -> RepositoryResult<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| RepositoryError::connection(format!("task join error: {e}")))?
}

/// Obtains a connection from the pool.
pub(super) fn get_conn(pool: &PgPool) -> RepositoryResult<PooledConn> {
    pool.get()
        .map_err(|e| RepositoryError::connection(e.to_string()))
}

/// Runs a blocking task and maps join errors into the caller's error type.
pub(super) async fn run_blocking_with<F, T, E, M>(f: F, map_err: M) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static,
    M: FnOnce(tokio::task::JoinError) -> E,
{
    tokio::task::spawn_blocking(f).await.map_err(map_err)?
}

/// Obtains a connection from the pool with a caller-provided error mapper.
pub(super) fn get_conn_with<E, M>(pool: &PgPool, map_err: M) -> Result<PooledConn, E>
where
    M: FnOnce(PoolError) -> E,
{
    pool.get().map_err(map_err)
}
