//! `PostgreSQL` adapters for hook engine persistence.

pub mod models;
pub mod repository;
pub mod schema;

pub use repository::{HookExecutionPgPool, PostgresHookExecutionLogRepository};
