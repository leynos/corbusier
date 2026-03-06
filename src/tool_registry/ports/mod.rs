//! Port contracts for MCP server lifecycle orchestration.

mod catalog;
mod host;
mod log_store;
mod policy;
mod repository;

pub use catalog::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult};
pub use host::{
    McpServerHost, McpServerHostError, McpServerHostResult, StartHostResult, ToolCallHostResult,
};
pub use log_store::{SweepContext, ToolLogStore, ToolLogStoreError, ToolLogStoreResult};
pub use policy::{ToolPolicyEnforcer, ToolPolicyError, ToolPolicyResult};
pub use repository::{
    McpServerRegistryError, McpServerRegistryRepository, McpServerRegistryResult,
};
