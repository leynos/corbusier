//! In-memory repository for MCP server registrations.

use crate::context::{RequestContext, TenantId};
use crate::tool_registry::{
    domain::{McpServerId, McpServerName, McpServerRegistration},
    ports::{McpServerRegistryError, McpServerRegistryRepository, McpServerRegistryResult},
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe in-memory MCP server registry repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryMcpServerRegistry {
    state: Arc<RwLock<HashMap<TenantId, InMemoryRegistryState>>>,
}

#[derive(Debug, Default)]
struct InMemoryRegistryState {
    servers: HashMap<McpServerId, McpServerRegistration>,
    name_index: HashMap<McpServerName, McpServerId>,
}

impl InMemoryMcpServerRegistry {
    /// Creates an empty in-memory registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl McpServerRegistryRepository for InMemoryMcpServerRegistry {
    async fn register(
        &self,
        ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerRegistryResult<()> {
        let mut tenants = self.state.write().map_err(|err| {
            McpServerRegistryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        let state = tenants.entry(ctx.tenant_id()).or_default();

        if state.servers.contains_key(&server.id()) {
            return Err(McpServerRegistryError::DuplicateServer(server.id()));
        }

        if state.name_index.contains_key(server.name()) {
            return Err(McpServerRegistryError::DuplicateServerName(
                server.name().clone(),
            ));
        }

        state.name_index.insert(server.name().clone(), server.id());
        state.servers.insert(server.id(), server.clone());
        Ok(())
    }

    async fn update(
        &self,
        ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerRegistryResult<()> {
        let mut tenants = self.state.write().map_err(|err| {
            McpServerRegistryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        let Some(state) = tenants.get_mut(&ctx.tenant_id()) else {
            return Err(McpServerRegistryError::NotFound(server.id()));
        };

        let stored_name = state
            .servers
            .get(&server.id())
            .ok_or(McpServerRegistryError::NotFound(server.id()))?
            .name()
            .clone();

        if *server.name() != stored_name {
            if let Some(&indexed_id) = state.name_index.get(server.name())
                && indexed_id != server.id()
            {
                return Err(McpServerRegistryError::DuplicateServerName(
                    server.name().clone(),
                ));
            }

            state.name_index.remove(&stored_name);
            state.name_index.insert(server.name().clone(), server.id());
        }

        state.servers.insert(server.id(), server.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> McpServerRegistryResult<Option<McpServerRegistration>> {
        let tenants = self.state.read().map_err(|err| {
            McpServerRegistryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(tenants
            .get(&ctx.tenant_id())
            .and_then(|state| state.servers.get(&server_id).cloned()))
    }

    async fn find_by_name(
        &self,
        ctx: &RequestContext,
        server_name: &McpServerName,
    ) -> McpServerRegistryResult<Option<McpServerRegistration>> {
        let tenants = self.state.read().map_err(|err| {
            McpServerRegistryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        let server = tenants.get(&ctx.tenant_id()).and_then(|state| {
            state
                .name_index
                .get(server_name)
                .and_then(|id| state.servers.get(id))
                .cloned()
        });
        Ok(server)
    }

    async fn list_all(
        &self,
        ctx: &RequestContext,
    ) -> McpServerRegistryResult<Vec<McpServerRegistration>> {
        let tenants = self.state.read().map_err(|err| {
            McpServerRegistryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(tenants
            .get(&ctx.tenant_id())
            .map(|state| state.servers.values().cloned().collect())
            .unwrap_or_default())
    }
}
