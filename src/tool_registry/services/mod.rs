//! Application services for MCP server lifecycle, tool discovery, and
//! call routing operations.

mod discovery;
mod lifecycle;

pub use discovery::{
    ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError,
    ToolDiscoveryRoutingServiceResult,
};
pub use lifecycle::{
    McpServerLifecycleService, McpServerLifecycleServiceError, RegisterMcpServerRequest,
};
