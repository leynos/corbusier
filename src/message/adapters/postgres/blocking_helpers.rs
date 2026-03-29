//! Blocking operation helpers for `PostgreSQL` repository.
//!
//! Provides utilities for offloading synchronous Diesel operations to a
//! dedicated thread pool, avoiding blocking the async executor.

pub use crate::postgres_support::PgPool;
pub(crate) use crate::postgres_support::{get_conn_with, run_blocking_with};
