//! Runtime host port for MCP server lifecycle operations.

use crate::tool_registry::domain::{
    McpServerHealthSnapshot, McpServerId, McpServerRegistration, McpToolDefinition,
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
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
    async fn start(&self, server: &McpServerRegistration) -> McpServerHostResult<StartHostResult>;

    /// Stops the server runtime.
    async fn stop(&self, server: &McpServerRegistration) -> McpServerHostResult<()>;

    /// Reports current health for a server.
    async fn health(
        &self,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<McpServerHealthSnapshot>;

    /// Lists tools exposed by the running server.
    async fn list_tools(
        &self,
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
        server: &McpServerRegistration,
        tool_name: &str,
        parameters: Value,
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

    /// Generic runtime failure.
    #[error("MCP host runtime error: {0}")]
    Runtime(Arc<dyn std::error::Error + Send + Sync>),
}

impl McpServerHostError {
    /// Wraps a runtime error from the host adapter.
    pub fn runtime(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Runtime(Arc::new(err))
    }
}
