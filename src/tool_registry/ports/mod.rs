//! Port contracts for MCP server lifecycle orchestration.

mod host;
mod repository;

pub use host::{McpServerHost, McpServerHostError, McpServerHostResult};
pub use repository::{
    McpServerRegistryError, McpServerRegistryRepository, McpServerRegistryResult,
};
