//! Service layer for MCP server lifecycle orchestration.

use crate::tool_registry::{
    domain::{
        McpServerHealthSnapshot, McpServerId, McpServerName, McpServerRegistration,
        McpToolDefinition, McpTransport, ToolRegistryDomainError,
    },
    ports::{
        McpServerHost, McpServerHostError, McpServerRegistryError, McpServerRegistryRepository,
    },
};
use mockable::Clock;
use std::{future::Future, pin::Pin, sync::Arc};
use thiserror::Error;

type LifecycleChangeFuture<'a> =
    Pin<Box<dyn Future<Output = McpServerLifecycleServiceResult<LifecycleChange>> + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleCompensationAction {
    Start,
    Stop,
}

#[derive(Debug, Clone)]
struct LifecycleChange {
    updated_server: McpServerRegistration,
    compensation: Option<LifecycleCompensationAction>,
}

impl LifecycleChange {
    const fn without_compensation(updated_server: McpServerRegistration) -> Self {
        Self {
            updated_server,
            compensation: None,
        }
    }

    const fn with_compensation(
        updated_server: McpServerRegistration,
        compensation: LifecycleCompensationAction,
    ) -> Self {
        Self {
            updated_server,
            compensation: Some(compensation),
        }
    }
}

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

/// Encapsulates the components of a lifecycle transition with compensation.
struct LifecycleTransition<HostOp, DomainMut> {
    host_operation: HostOp,
    domain_mutation: DomainMut,
    compensation: LifecycleCompensationAction,
}

impl<HostOp, DomainMut> LifecycleTransition<HostOp, DomainMut> {
    const fn new(
        host_operation: HostOp,
        domain_mutation: DomainMut,
        compensation: LifecycleCompensationAction,
    ) -> Self {
        Self {
            host_operation,
            domain_mutation,
            compensation,
        }
    }
}

impl<R, H, C> McpServerLifecycleService<R, H, C>
where
    R: McpServerRegistryRepository,
    H: McpServerHost + 'static,
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

    async fn execute_lifecycle_change<F>(
        &self,
        server_id: McpServerId,
        apply_change: F,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration>
    where
        F: for<'a> FnOnce(&'a McpServerRegistration, &'a Self) -> LifecycleChangeFuture<'a>,
    {
        let server = self.find_server_or_error(server_id).await?;
        let change = apply_change(&server, self).await?;
        if let Err(repository_error) = self.repository.update(&change.updated_server).await {
            if let Some(compensation_error) = self
                .apply_compensation_if_needed(
                    change.compensation,
                    &change.updated_server,
                    &repository_error,
                )
                .await
            {
                return Err(compensation_error);
            }
            return Err(McpServerLifecycleServiceError::Repository(repository_error));
        }
        Ok(change.updated_server)
    }

    async fn apply_compensation_if_needed(
        &self,
        compensation: Option<LifecycleCompensationAction>,
        server: &McpServerRegistration,
        repository_error: &McpServerRegistryError,
    ) -> Option<McpServerLifecycleServiceError> {
        let compensation_action = compensation?;
        let compensation_result = match compensation_action {
            LifecycleCompensationAction::Start => self.host.start(server).await,
            LifecycleCompensationAction::Stop => self.host.stop(server).await,
        };
        compensation_result.err().map(|host_error| {
            let combined_error = std::io::Error::other(format!(
                "lifecycle persistence failed: {repository_error}; compensation failed: {host_error}"
            ));
            McpServerLifecycleServiceError::Host(McpServerHostError::runtime(combined_error))
        })
    }

    /// Helper for lifecycle transitions with compensation.
    async fn execute_transition_with_compensation<HostOp, DomainMut>(
        &self,
        server_id: McpServerId,
        transition: LifecycleTransition<HostOp, DomainMut>,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration>
    where
        HostOp: FnOnce(
                McpServerRegistration,
                Arc<H>,
            )
                -> Pin<Box<dyn Future<Output = Result<(), McpServerHostError>> + Send>>
            + 'static,
        DomainMut:
            FnOnce(&mut McpServerRegistration, &C) -> Result<(), ToolRegistryDomainError> + 'static,
    {
        let LifecycleTransition {
            host_operation,
            domain_mutation,
            compensation,
        } = transition;
        self.execute_lifecycle_change(server_id, move |server, service| {
            Box::pin(async move {
                let mut updated_server = server.clone();
                domain_mutation(&mut updated_server, &*service.clock)?;
                host_operation(server.clone(), Arc::clone(&service.host)).await?;
                Ok(LifecycleChange::with_compensation(
                    updated_server,
                    compensation,
                ))
            })
        })
        .await
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
        let transition = LifecycleTransition::new(
            |server: McpServerRegistration,
             host: Arc<H>|
             -> Pin<Box<dyn Future<Output = Result<(), McpServerHostError>> + Send>> {
                Box::pin(async move { host.start(&server).await })
            },
            |server: &mut McpServerRegistration, clock: &C| {
                server.mark_started(McpServerHealthSnapshot::unknown(clock.utc()), clock)
            },
            LifecycleCompensationAction::Stop,
        );
        self.execute_transition_with_compensation(server_id, transition)
            .await?;
        self.refresh_health(server_id).await
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
        let transition = LifecycleTransition::new(
            |server: McpServerRegistration,
             host: Arc<H>|
             -> Pin<Box<dyn Future<Output = Result<(), McpServerHostError>> + Send>> {
                Box::pin(async move { host.stop(&server).await })
            },
            |server: &mut McpServerRegistration, clock: &C| server.mark_stopped(clock),
            LifecycleCompensationAction::Start,
        );
        self.execute_transition_with_compensation(server_id, transition)
            .await
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
        self.execute_lifecycle_change(server_id, |server, service| {
            Box::pin(async move {
                let health_snapshot = service.host.health(server).await?;
                let mut refreshed_server = server.clone();
                refreshed_server.update_health(health_snapshot, &*service.clock);
                Ok(LifecycleChange::without_compensation(refreshed_server))
            })
        })
        .await
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
mod tests;
