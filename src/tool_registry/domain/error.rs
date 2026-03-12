//! Error types for MCP server domain validation and parsing.

use super::McpServerId;
use super::routing::ToolCallId;
use thiserror::Error;

/// Errors returned while constructing tool registry domain values.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ToolRegistryDomainError {
    /// The server name is empty after trimming.
    #[error("MCP server name must not be empty")]
    EmptyServerName,

    /// The server name contains characters outside `[a-z0-9_]`.
    #[error(
        "MCP server name '{0}' contains invalid characters (only lowercase alphanumeric and underscores allowed)"
    )]
    InvalidServerName(String),

    /// The server name exceeds the 100-character storage limit.
    #[error("MCP server name exceeds 100 character limit: {0}")]
    ServerNameTooLong(String),

    /// The STDIO command is empty.
    #[error("STDIO command must not be empty")]
    EmptyStdioCommand,

    /// The STDIO working directory is empty after trimming.
    #[error("STDIO working directory must not be empty when provided")]
    EmptyWorkingDirectory,

    /// The HTTP+SSE base URL is empty.
    #[error("HTTP+SSE base URL must not be empty")]
    EmptyHttpSseBaseUrl,

    /// The HTTP+SSE base URL does not have an `http://` or `https://` prefix.
    #[error("HTTP+SSE base URL '{0}' must start with 'http://' or 'https://'")]
    InvalidHttpSseBaseUrl(String),

    /// A tool definition name is empty after trimming.
    #[error("tool name must not be empty")]
    EmptyToolName,

    /// A tool definition description is empty after trimming.
    #[error("tool description must not be empty")]
    EmptyToolDescription,

    /// Transitioning between two lifecycle states is invalid.
    #[error("invalid MCP server lifecycle transition: {from} -> {to}")]
    InvalidLifecycleTransition {
        /// Current lifecycle state.
        from: String,
        /// Requested target lifecycle state.
        to: String,
    },

    /// Tool queries require the server lifecycle state to be `running`.
    #[error("MCP server {server_id} is not running (current state: {state})")]
    ToolQueryRequiresRunning {
        /// Server identifier.
        server_id: McpServerId,
        /// Lifecycle state in canonical string form.
        state: String,
    },

    /// No tool with the given name exists in the catalog.
    #[error("tool not found: {0}")]
    ToolNotFound(String),

    /// The tool exists but is not currently available for invocation.
    #[error("tool '{tool_name}' on server {server_id} is unavailable")]
    ToolUnavailable {
        /// Name of the unavailable tool.
        tool_name: String,
        /// Server hosting the tool.
        server_id: McpServerId,
    },

    /// Tool call parameters do not satisfy the tool's input schema.
    #[error("schema validation failed for tool '{tool_name}': {reason}")]
    SchemaValidationFailed {
        /// Name of the tool whose schema was violated.
        tool_name: String,
        /// Human-readable validation failure description.
        reason: String,
    },

    /// A policy enforcement point denied the tool call.
    #[error("policy denied tool call to '{tool_name}': {reason}")]
    PolicyDenied {
        /// Name of the denied tool.
        tool_name: String,
        /// Human-readable denial reason.
        reason: String,
    },

    /// The tool call timed out before completion.
    #[error("tool call to '{tool_name}' timed out (call_id: {call_id})")]
    ToolCallTimeout {
        /// Name of the timed-out tool.
        tool_name: String,
        /// Identifier of the timed-out call.
        call_id: ToolCallId,
    },

    /// Multiple servers advertise a tool with the same name.
    #[error("ambiguous tool name '{tool_name}' found on {server_count} servers")]
    AmbiguousToolName {
        /// The conflicting tool name.
        tool_name: String,
        /// Number of servers advertising this tool name.
        server_count: usize,
    },
}

/// Error returned while parsing lifecycle state from persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown MCP server lifecycle state: {0}")]
pub struct ParseMcpServerLifecycleStateError(pub String);

/// Error returned while parsing health status from persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown MCP server health status: {0}")]
pub struct ParseMcpServerHealthStatusError(pub String);
