//! Application services for MCP server lifecycle, tool discovery, and
//! call routing operations.

mod discovery;
mod lifecycle;

pub use discovery::{
    ServicePorts, ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError,
    ToolDiscoveryRoutingServiceResult,
};
pub use lifecycle::{
    LifecycleStartResult, McpServerLifecycleService, McpServerLifecycleServiceError,
    RegisterMcpServerRequest,
};
