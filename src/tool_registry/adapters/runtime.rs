//! In-memory runtime host adapter for MCP server lifecycle tests.

use crate::tool_registry::{
    domain::{
        McpServerHealthSnapshot, McpServerId, McpServerName, McpServerRegistration,
        McpToolDefinition,
    },
    ports::{McpServerHost, McpServerHostError, McpServerHostResult},
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

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
}

impl InMemoryMcpServerHost {
    /// Creates an empty in-memory host.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
        let mut state = self
            .state
            .write()
            .map_err(|err| McpServerHostError::runtime(std::io::Error::other(err.to_string())))?;
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
        let mut state = self
            .state
            .write()
            .map_err(|err| McpServerHostError::runtime(std::io::Error::other(err.to_string())))?;
        state
            .unhealthy_servers
            .insert(server_id, message.into().trim().to_owned());
        Ok(())
    }
}

#[async_trait]
impl McpServerHost for InMemoryMcpServerHost {
    async fn start(&self, server: &McpServerRegistration) -> McpServerHostResult<()> {
        let mut state = self
            .state
            .write()
            .map_err(|err| McpServerHostError::runtime(std::io::Error::other(err.to_string())))?;
        state.running_servers.insert(server.id());
        state.unhealthy_servers.remove(&server.id());
        Ok(())
    }

    async fn stop(&self, server: &McpServerRegistration) -> McpServerHostResult<()> {
        let mut state = self
            .state
            .write()
            .map_err(|err| McpServerHostError::runtime(std::io::Error::other(err.to_string())))?;
        state.running_servers.remove(&server.id());
        state.unhealthy_servers.remove(&server.id());
        Ok(())
    }

    async fn health(
        &self,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<McpServerHealthSnapshot> {
        let state = self
            .state
            .read()
            .map_err(|err| McpServerHostError::runtime(std::io::Error::other(err.to_string())))?;

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
        let state = self
            .state
            .read()
            .map_err(|err| McpServerHostError::runtime(std::io::Error::other(err.to_string())))?;

        if !state.running_servers.contains(&server.id()) {
            return Err(McpServerHostError::NotRunning(server.id()));
        }

        Ok(state
            .tool_catalogs
            .get(server.name())
            .cloned()
            .unwrap_or_default())
    }
}
