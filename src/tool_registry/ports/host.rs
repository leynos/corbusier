//! Runtime host port for MCP server lifecycle operations.

use crate::tool_registry::domain::{
    McpServerHealthSnapshot, McpServerId, McpServerRegistration, McpToolDefinition,
};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for runtime MCP host operations.
pub type McpServerHostResult<T> = Result<T, McpServerHostError>;

/// Runtime control contract for MCP server lifecycle and tool discovery.
#[async_trait]
pub trait McpServerHost: Send + Sync {
    /// Starts the server runtime.
    async fn start(&self, server: &McpServerRegistration) -> McpServerHostResult<()>;

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
