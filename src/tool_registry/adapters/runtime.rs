//! In-memory runtime host adapter for MCP server lifecycle tests.

use crate::tool_registry::{
    domain::{
        McpServerHealthSnapshot, McpServerId, McpServerName, McpServerRegistration,
        McpToolDefinition,
    },
    ports::{
        McpServerHost, McpServerHostError, McpServerHostResult, StartHostResult, ToolCallHostResult,
    },
};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// In-memory MCP server host adapter.
///
/// This adapter models lifecycle behaviour without spawning external
/// processes. It is suitable for unit and integration tests and for local
/// deterministic orchestration flows.
#[derive(Debug, Clone, Default)]
pub struct InMemoryMcpServerHost {
    state: Arc<RwLock<InMemoryHostState>>,
}

#[derive(Debug, Default)]
struct InMemoryHostState {
    running_servers: HashSet<McpServerId>,
    unhealthy_servers: HashMap<McpServerId, String>,
    tool_catalogs: HashMap<McpServerName, Vec<McpToolDefinition>>,
    tool_call_results: HashMap<(McpServerName, String), Value>,
    tool_call_stderr: HashMap<(McpServerName, String), bytes::Bytes>,
    startup_stderr: HashMap<McpServerName, bytes::Bytes>,
}

impl InMemoryMcpServerHost {
    /// Creates an empty in-memory host.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn read_state(&self) -> McpServerHostResult<RwLockReadGuard<'_, InMemoryHostState>> {
        self.state
            .read()
            .map_err(|err| McpServerHostError::runtime(std::io::Error::other(err.to_string())))
    }

    fn write_state(&self) -> McpServerHostResult<RwLockWriteGuard<'_, InMemoryHostState>> {
        self.state
            .write()
            .map_err(|err| McpServerHostError::runtime(std::io::Error::other(err.to_string())))
    }

    /// Associates a tool catalog with a server name.
    ///
    /// Existing catalog entries are replaced.
    ///
    /// # Errors
    ///
    /// Returns host runtime errors when lock acquisition fails.
    pub fn set_tool_catalog(
        &self,
        server_name: McpServerName,
        tools: Vec<McpToolDefinition>,
    ) -> McpServerHostResult<()> {
        let mut state = self.write_state()?;
        state.tool_catalogs.insert(server_name, tools);
        Ok(())
    }

    /// Marks a running server as unhealthy with a diagnostic message.
    ///
    /// # Errors
    ///
    /// Returns host runtime errors when lock acquisition fails.
    pub fn set_unhealthy(
        &self,
        server_id: McpServerId,
        message: impl Into<String>,
    ) -> McpServerHostResult<()> {
        let mut state = self.write_state()?;
        state
            .unhealthy_servers
            .insert(server_id, message.into().trim().to_owned());
        Ok(())
    }

    /// Configures the result that `call_tool` will return for a given
    /// server and tool name combination.
    ///
    /// # Errors
    ///
    /// Returns host runtime errors when lock acquisition fails.
    pub fn set_tool_call_result(
        &self,
        server_name: McpServerName,
        tool_name: impl Into<String>,
        result: Value,
    ) -> McpServerHostResult<()> {
        let mut state = self.write_state()?;
        state
            .tool_call_results
            .insert((server_name, tool_name.into()), result);
        Ok(())
    }

    /// Configures stderr output that `call_tool` will include for a
    /// given server and tool name combination.
    ///
    /// # Errors
    ///
    /// Returns host runtime errors when lock acquisition fails.
    pub fn set_tool_call_stderr(
        &self,
        server_name: McpServerName,
        tool_name: impl Into<String>,
        stderr: bytes::Bytes,
    ) -> McpServerHostResult<()> {
        let mut state = self.write_state()?;
        state
            .tool_call_stderr
            .insert((server_name, tool_name.into()), stderr);
        Ok(())
    }

    /// Configures stderr output that `start` will return for a given
    /// server name.
    ///
    /// # Errors
    ///
    /// Returns host runtime errors when lock acquisition fails.
    pub fn set_startup_stderr(
        &self,
        server_name: McpServerName,
        stderr: bytes::Bytes,
    ) -> McpServerHostResult<()> {
        let mut state = self.write_state()?;
        state.startup_stderr.insert(server_name, stderr);
        Ok(())
    }
}

#[async_trait]
impl McpServerHost for InMemoryMcpServerHost {
    async fn start(&self, server: &McpServerRegistration) -> McpServerHostResult<StartHostResult> {
        let mut state = self.write_state()?;
        state.running_servers.insert(server.id());
        state.unhealthy_servers.remove(&server.id());
        let stderr_output = state.startup_stderr.get(server.name()).cloned();
        Ok(StartHostResult { stderr_output })
    }

    async fn stop(&self, server: &McpServerRegistration) -> McpServerHostResult<()> {
        let mut state = self.write_state()?;
        state.running_servers.remove(&server.id());
        state.unhealthy_servers.remove(&server.id());
        Ok(())
    }

    async fn health(
        &self,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<McpServerHealthSnapshot> {
        let state = self.read_state()?;

        let checked_at = Utc::now();
        if !state.running_servers.contains(&server.id()) {
            return Ok(McpServerHealthSnapshot::unknown(checked_at));
        }

        if let Some(message) = state.unhealthy_servers.get(&server.id()) {
            return Ok(McpServerHealthSnapshot::unhealthy(
                checked_at,
                message.clone(),
            ));
        }

        Ok(McpServerHealthSnapshot::healthy(checked_at))
    }

    async fn list_tools(
        &self,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<Vec<McpToolDefinition>> {
        let state = self.read_state()?;

        if !state.running_servers.contains(&server.id()) {
            return Err(McpServerHostError::NotRunning(server.id()));
        }

        Ok(state
            .tool_catalogs
            .get(server.name())
            .cloned()
            .unwrap_or_default())
    }

    async fn call_tool(
        &self,
        server: &McpServerRegistration,
        tool_name: &str,
        _parameters: Value,
    ) -> McpServerHostResult<ToolCallHostResult> {
        let state = self.read_state()?;

        if !state.running_servers.contains(&server.id()) {
            return Err(McpServerHostError::NotRunning(server.id()));
        }

        let key = (server.name().clone(), tool_name.to_owned());
        let content = state.tool_call_results.get(&key).cloned().ok_or_else(|| {
            McpServerHostError::ToolCallFailed {
                server_id: server.id(),
                tool_name: tool_name.to_owned(),
                reason: "no result configured for this tool".to_owned(),
            }
        })?;
        let stderr_output = state.tool_call_stderr.get(&key).cloned();

        Ok(ToolCallHostResult {
            content,
            stderr_output,
        })
    }
}
