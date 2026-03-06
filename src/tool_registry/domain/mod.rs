//! Domain model for MCP server lifecycle and tool discovery.
//!
//! The tool registry domain models MCP server identity, transport
//! configuration, lifecycle and health states, discovered tool metadata,
//! tool catalog entries, call routing, audit trails, policy decisions,
//! parameter validation, and stderr log capture with retention policies.
//! Infrastructure concerns remain outside this boundary.

mod audit;
mod catalog;
mod error;
mod health;
mod ids;
mod log_capture;
mod policy;
pub mod routing;
mod server;
mod tool;
mod transport;
pub mod validation;

pub use audit::ToolCallAuditRecord;
pub use catalog::{CatalogEntry, CatalogEntryId, PersistedCatalogEntryData};
pub use error::{
    ParseMcpServerHealthStatusError, ParseMcpServerLifecycleStateError, ToolRegistryDomainError,
};
pub use health::{McpServerHealthSnapshot, McpServerHealthStatus};
pub use ids::{McpServerId, McpServerName};
pub use log_capture::{LogEntryId, LogEntryKind, LogEntryMetadata, LogRetentionPolicy};
pub use policy::PolicyDecision;
pub use routing::{ToolCallId, ToolCallOutcome, ToolCallRequest, ToolCallResult, ToolCallTiming};
pub use server::{McpServerLifecycleState, McpServerRegistration, PersistedMcpServerData};
pub use tool::McpToolDefinition;
pub use transport::{HttpSseTransportConfig, McpTransport, StdioTransportConfig};
