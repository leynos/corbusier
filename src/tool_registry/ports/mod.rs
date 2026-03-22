//! Port contracts for MCP server lifecycle orchestration.

mod catalog;
mod governance;
mod host;
mod log_store;
mod repository;

pub use catalog::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult};
pub use governance::{ToolExecutionGovernance, ToolGovernanceError, ToolGovernanceResult};
pub use host::{
    McpServerHost, McpServerHostError, McpServerHostResult, StartHostResult, ToolCallHostResult,
};
pub use log_store::{
    StoreLogRequest, SweepContext, ToolLogStore, ToolLogStoreError, ToolLogStoreResult,
};
pub use repository::{
    McpServerRegistryError, McpServerRegistryRepository, McpServerRegistryResult,
};
