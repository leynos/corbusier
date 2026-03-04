//! Domain model for MCP server lifecycle and tool discovery.
//!
//! The tool registry domain models MCP server identity, transport
//! configuration, lifecycle and health states, and discovered tool metadata.
//! Infrastructure concerns remain outside this boundary.

mod error;
mod health;
mod ids;
mod server;
mod tool;
mod transport;

pub use error::{
    ParseMcpServerHealthStatusError, ParseMcpServerLifecycleStateError, ToolRegistryDomainError,
};
pub use health::{McpServerHealthSnapshot, McpServerHealthStatus};
pub use ids::{McpServerId, McpServerName};
pub use server::{McpServerLifecycleState, McpServerRegistration, PersistedMcpServerData};
pub use tool::McpToolDefinition;
pub use transport::{HttpSseTransportConfig, McpTransport, StdioTransportConfig};
