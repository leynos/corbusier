//! Error types for MCP server domain validation and parsing.

use super::McpServerId;
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
}

/// Error returned while parsing lifecycle state from persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown MCP server lifecycle state: {0}")]
pub struct ParseMcpServerLifecycleStateError(pub String);

/// Error returned while parsing health status from persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown MCP server health status: {0}")]
pub struct ParseMcpServerHealthStatusError(pub String);
