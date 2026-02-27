//! `PostgreSQL` adapters for agent backend registry persistence.

mod models;
mod repository;
mod schema;

pub use repository::{BackendPgPool, PostgresBackendRegistry};
