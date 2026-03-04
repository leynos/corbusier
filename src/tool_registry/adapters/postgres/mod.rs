//! `PostgreSQL` adapters for MCP server registry persistence.

mod models;
mod repository;
mod schema;

pub use repository::{McpServerPgPool, PostgresMcpServerRegistry};
