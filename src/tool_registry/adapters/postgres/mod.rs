//! `PostgreSQL` adapters for MCP server registry persistence.

mod catalog_models;
mod catalog_repository;
mod catalog_schema;
mod models;
mod repository;
mod schema;

pub use catalog_repository::PostgresToolCatalog;
pub use repository::{McpServerPgPool, PostgresMcpServerRegistry};
