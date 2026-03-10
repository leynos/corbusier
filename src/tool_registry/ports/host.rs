//! Runtime host port for MCP server lifecycle operations.

use crate::context::RequestContext;
use crate::tool_registry::domain::{
    McpServerHealthSnapshot, McpServerId, McpServerRegistration, McpToolDefinition, ToolCallRequest,
};
use async_trait::async_trait;
use serde_json::Value;
use thiserror::Error;

/// Result type for runtime MCP host operations.
pub type McpServerHostResult<T> = Result<T, McpServerHostError>;

/// Result of starting an MCP server, including captured stderr.
#[derive(Debug, Clone, Default)]
pub struct StartHostResult {
    /// Stderr output captured during server startup, if any.
    pub stderr_output: Option<bytes::Bytes>,
}

/// Result of a tool call, including content and captured stderr.
#[derive(Debug, Clone)]
pub struct ToolCallHostResult {
    /// Content returned by the tool.
    pub content: Value,
    /// Stderr output captured during the tool call, if any.
    pub stderr_output: Option<bytes::Bytes>,
}

/// Runtime control contract for MCP server lifecycle and tool discovery.
#[async_trait]
pub trait McpServerHost: Send + Sync {
    /// Starts the server runtime.
    ///
    /// Returns a [`StartHostResult`] containing any captured startup
    /// stderr output.
    async fn start(
        &self,
        ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<StartHostResult>;

    /// Stops the server runtime.
    async fn stop(
        &self,
        ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<()>;

    /// Reports current health for a server.
    async fn health(
        &self,
        ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<McpServerHealthSnapshot>;

    /// Lists tools exposed by the running server.
    async fn list_tools(
        &self,
        ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<Vec<McpToolDefinition>>;

    /// Invokes a tool on the running server.
    ///
    /// Returns a [`ToolCallHostResult`] containing the tool's output
    /// content and any captured stderr.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerHostError::NotRunning`] when the server is not
    /// running, [`McpServerHostError::ToolCallFailed`] when the tool
    /// invocation fails, or [`McpServerHostError::ToolCallTimeout`] when
    /// the call exceeds the timeout.
    async fn call_tool(
        &self,
        ctx: &RequestContext,
        server: &McpServerRegistration,
        request: &ToolCallRequest,
    ) -> McpServerHostResult<ToolCallHostResult>;
}

/// Errors returned by runtime MCP host adapters.
#[derive(Debug, Clone, Error)]
pub enum McpServerHostError {
    /// The server is not currently running.
    #[error("MCP server {0} is not running")]
    NotRunning(McpServerId),

    /// The server transport is unsupported by the host adapter.
    #[error("unsupported transport for MCP server {server_id}: {reason}")]
    UnsupportedTransport {
        /// Server identifier.
        server_id: McpServerId,
        /// Reason string.
        reason: String,
    },

    /// A tool call invocation failed.
    #[error("tool call to '{tool_name}' failed on server {server_id}: {reason}")]
    ToolCallFailed {
        /// Server identifier.
        server_id: McpServerId,
        /// Name of the tool that failed.
        tool_name: String,
        /// Reason for the failure.
        reason: String,
    },

    /// A tool call timed out.
    #[error("tool call to '{tool_name}' timed out on server {server_id}")]
    ToolCallTimeout {
        /// Server identifier.
        server_id: McpServerId,
        /// Name of the timed-out tool.
        tool_name: String,
    },

    /// The MCP host process failed to spawn.
    #[error("MCP host process failed to spawn for server {server_id}: {reason}")]
    ProcessSpawnFailed {
        /// Server identifier.
        server_id: McpServerId,
        /// Reason string.
        reason: String,
    },

    /// Communication with the MCP host process failed.
    #[error("MCP host communication error for server {server_id}: {reason}")]
    CommunicationError {
        /// Server identifier.
        server_id: McpServerId,
        /// Reason string.
        reason: String,
    },

    /// The MCP protocol exchange failed.
    #[error("MCP protocol error for server {server_id}: {reason}")]
    ProtocolError {
        /// Server identifier.
        server_id: McpServerId,
        /// Reason string.
        reason: String,
    },
}
