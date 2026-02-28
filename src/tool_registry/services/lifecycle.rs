//! Service layer for MCP server lifecycle orchestration.

use crate::tool_registry::{
    domain::{
        McpServerId, McpServerName, McpServerRegistration, McpToolDefinition, McpTransport,
        ToolRegistryDomainError,
    },
    ports::{
        McpServerHost, McpServerHostError, McpServerRegistryError, McpServerRegistryRepository,
    },
};
use mockable::Clock;
use std::sync::Arc;
use thiserror::Error;

/// Request payload for registering an MCP server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterMcpServerRequest {
    /// Human-readable unique server name.
    pub name: String,
    /// Transport configuration.
    pub transport: McpTransport,
}

impl RegisterMcpServerRequest {
    /// Creates a registration request.
    #[must_use]
    pub fn new(name: impl Into<String>, transport: McpTransport) -> Self {
        Self {
            name: name.into(),
            transport,
        }
    }
}

/// Service-level errors for MCP server lifecycle operations.
#[derive(Debug, Error)]
pub enum McpServerLifecycleServiceError {
    /// Domain validation failed.
    #[error(transparent)]
    Domain(#[from] ToolRegistryDomainError),
    /// Repository operation failed.
    #[error(transparent)]
    Repository(#[from] McpServerRegistryError),
    /// Host operation failed.
    #[error(transparent)]
    Host(#[from] McpServerHostError),
    /// No server exists with the given identifier.
    #[error("MCP server {0} not found")]
    NotFound(McpServerId),
}

/// Result type for lifecycle service operations.
pub type McpServerLifecycleServiceResult<T> = Result<T, McpServerLifecycleServiceError>;

/// MCP server lifecycle orchestration service.
#[derive(Clone)]
pub struct McpServerLifecycleService<R, H, C>
where
    R: McpServerRegistryRepository,
    H: McpServerHost,
    C: Clock + Send + Sync,
{
    repository: Arc<R>,
    host: Arc<H>,
    clock: Arc<C>,
}

impl<R, H, C> McpServerLifecycleService<R, H, C>
where
    R: McpServerRegistryRepository,
    H: McpServerHost,
    C: Clock + Send + Sync,
{
    /// Creates a new lifecycle service.
    #[must_use]
    pub const fn new(repository: Arc<R>, host: Arc<H>, clock: Arc<C>) -> Self {
        Self {
            repository,
            host,
            clock,
        }
    }

    async fn find_server_or_error(
        &self,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        self.repository
            .find_by_id(server_id)
            .await?
            .ok_or(McpServerLifecycleServiceError::NotFound(server_id))
    }

    /// Registers a new MCP server.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerLifecycleServiceError`] when validation fails or
    /// persistence rejects registration.
    pub async fn register(
        &self,
        request: RegisterMcpServerRequest,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        let server_name = McpServerName::new(request.name)?;
        let registration = McpServerRegistration::new(server_name, request.transport, &*self.clock);
        self.repository.register(&registration).await?;
        Ok(registration)
    }

    /// Starts a registered MCP server.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerLifecycleServiceError::NotFound`] when no server has
    /// the given ID, domain errors for invalid lifecycle transitions, or host
    /// and persistence errors.
    pub async fn start(
        &self,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        let mut server = self.find_server_or_error(server_id).await?;
        self.host.start(&server).await?;
        let health_snapshot = self.host.health(&server).await?;
        server.mark_started(health_snapshot, &*self.clock)?;
        self.repository.update(&server).await?;
        Ok(server)
    }

    /// Stops a registered MCP server.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerLifecycleServiceError::NotFound`] when no server has
    /// the given ID, domain errors for invalid lifecycle transitions, or host
    /// and persistence errors.
    pub async fn stop(
        &self,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        let mut server = self.find_server_or_error(server_id).await?;
        self.host.stop(&server).await?;
        server.mark_stopped(&*self.clock)?;
        self.repository.update(&server).await?;
        Ok(server)
    }

    /// Refreshes and persists server health.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerLifecycleServiceError::NotFound`] when no server has
    /// the given ID, or host and persistence errors.
    pub async fn refresh_health(
        &self,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        let mut server = self.find_server_or_error(server_id).await?;
        let health_snapshot = self.host.health(&server).await?;
        server.update_health(health_snapshot, &*self.clock);
        self.repository.update(&server).await?;
        Ok(server)
    }

    /// Lists all registered MCP servers.
    ///
    /// # Errors
    ///
    /// Returns persistence-layer errors from the repository.
    pub async fn list_all(&self) -> McpServerLifecycleServiceResult<Vec<McpServerRegistration>> {
        Ok(self.repository.list_all().await?)
    }

    /// Finds a registered server by name.
    ///
    /// # Errors
    ///
    /// Returns domain validation errors when the name is invalid and
    /// persistence errors from the repository.
    pub async fn find_by_name(
        &self,
        server_name: &str,
    ) -> McpServerLifecycleServiceResult<Option<McpServerRegistration>> {
        let validated_name = McpServerName::new(server_name)?;
        Ok(self.repository.find_by_name(&validated_name).await?)
    }

    /// Returns tools exposed by a running server.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerLifecycleServiceError::NotFound`] when no server has
    /// the given ID, domain errors when lifecycle state does not allow querying
    /// tools, or host errors.
    pub async fn list_tools(
        &self,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<Vec<McpToolDefinition>> {
        let server = self.find_server_or_error(server_id).await?;
        server.ensure_can_query_tools()?;
        Ok(self.host.list_tools(&server).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_registry::{
        adapters::{InMemoryMcpServerHost, memory::InMemoryMcpServerRegistry},
        domain::McpToolDefinition,
    };
    use mockable::DefaultClock;
    use rstest::rstest;
    use serde_json::json;

    type TestService =
        McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

    fn build_service() -> (Arc<InMemoryMcpServerHost>, TestService) {
        let host = Arc::new(InMemoryMcpServerHost::new());
        let service = McpServerLifecycleService::new(
            Arc::new(InMemoryMcpServerRegistry::new()),
            host.clone(),
            Arc::new(DefaultClock),
        );
        (host, service)
    }

    fn stdio_request(name: &str) -> RegisterMcpServerRequest {
        RegisterMcpServerRequest::new(
            name,
            McpTransport::stdio("mcp-server").expect("valid stdio transport"),
        )
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn register_and_find_by_name() {
        let (_, service) = build_service();

        let registered = service
            .register(stdio_request("workspace_tools"))
            .await
            .expect("registration should succeed");

        let found = service
            .find_by_name("workspace_tools")
            .await
            .expect("lookup should succeed")
            .expect("server should exist");

        assert_eq!(found.id(), registered.id());
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn start_unknown_server_returns_not_found() {
        let (_, service) = build_service();

        let result = service.start(McpServerId::new()).await;

        assert!(matches!(
            result,
            Err(McpServerLifecycleServiceError::NotFound(_))
        ));
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn list_tools_requires_running_server() {
        let (_, service) = build_service();
        let registered = service
            .register(stdio_request("workspace_tools"))
            .await
            .expect("registration should succeed");

        let result = service.list_tools(registered.id()).await;

        assert!(matches!(
            result,
            Err(McpServerLifecycleServiceError::Domain(
                ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
            ))
        ));
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn start_and_list_tools() {
        let (host, service) = build_service();

        let tool = McpToolDefinition::new(
            "read_file",
            "Reads a file from the workspace",
            json!({"type": "object", "properties": {"path": {"type": "string"}}}),
        )
        .expect("tool definition should be valid");

        host.set_tool_catalog(
            McpServerName::new("workspace_tools").expect("valid name"),
            vec![tool],
        )
        .expect("catalog update should succeed");

        let registered = service
            .register(stdio_request("workspace_tools"))
            .await
            .expect("registration should succeed");
        service
            .start(registered.id())
            .await
            .expect("start should succeed");

        let tools = service
            .list_tools(registered.id())
            .await
            .expect("tool listing should succeed");

        assert_eq!(tools.len(), 1);
        let first = tools.first().expect("tool should exist");
        assert_eq!(first.name(), "read_file");
    }
}
