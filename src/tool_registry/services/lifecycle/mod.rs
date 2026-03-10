//! Service layer for MCP server lifecycle orchestration.

mod transitions;

use crate::{
    context::RequestContext,
    tool_registry::{
        domain::{
            McpServerHealthSnapshot, McpServerId, McpServerName, McpServerRegistration,
            McpToolDefinition, McpTransport, ToolRegistryDomainError,
        },
        ports::{
            McpServerHost, McpServerHostError, McpServerRegistryError, McpServerRegistryRepository,
        },
    },
};
use mockable::Clock;
use std::{future::Future, pin::Pin, sync::Arc};
use thiserror::Error;
use transitions::{
    LifecycleChange, LifecycleCompensationAction, LifecycleHostAction, LifecycleTransition,
};

type LifecycleChangeFuture<'a> =
    Pin<Box<dyn Future<Output = McpServerLifecycleServiceResult<LifecycleChange>> + 'a>>;

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

/// Result of starting an MCP server, including captured startup stderr.
#[derive(Debug, Clone)]
pub struct LifecycleStartResult {
    /// The updated server registration after starting.
    pub server: McpServerRegistration,
    /// Stderr output captured during server startup, if any.
    pub startup_stderr: Option<bytes::Bytes>,
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
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        self.repository
            .find_by_id(ctx, server_id)
            .await?
            .ok_or(McpServerLifecycleServiceError::NotFound(server_id))
    }

    async fn execute_lifecycle_change<F>(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        apply_change: F,
    ) -> McpServerLifecycleServiceResult<LifecycleChange>
    where
        F: for<'a> FnOnce(&'a McpServerRegistration, &'a Self) -> LifecycleChangeFuture<'a>,
    {
        let server = self.find_server_or_error(ctx, server_id).await?;
        let change = apply_change(&server, self).await?;
        if let Err(repository_error) = self.repository.update(ctx, &change.updated_server).await {
            if let Some(compensation_error) = self
                .apply_compensation_if_needed(
                    ctx,
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
        Ok(change)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "RequestContext plumbing adds one parameter beyond the natural arity"
    )]
    async fn apply_compensation_if_needed(
        &self,
        ctx: &RequestContext,
        compensation: Option<LifecycleCompensationAction>,
        server: &McpServerRegistration,
        repository_error: &McpServerRegistryError,
    ) -> Option<McpServerLifecycleServiceError> {
        let compensation_action = compensation?;
        let compensation_result = match compensation_action {
            LifecycleCompensationAction::Start => self.host.start(ctx, server).await.map(|_| ()),
            LifecycleCompensationAction::Stop => self.host.stop(ctx, server).await,
        };
        compensation_result.err().map(|host_error| {
            let combined_error = std::io::Error::other(format!(
                "lifecycle persistence failed: {repository_error}; compensation failed: {host_error}"
            ));
            McpServerLifecycleServiceError::Host(McpServerHostError::runtime(combined_error))
        })
    }

    /// Helper for lifecycle transitions with compensation.
    async fn execute_transition_with_compensation<DomainMut>(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        transition: LifecycleTransition<DomainMut>,
    ) -> McpServerLifecycleServiceResult<LifecycleChange>
    where
        DomainMut:
            FnOnce(&mut McpServerRegistration, &C) -> Result<(), ToolRegistryDomainError> + 'static,
    {
        let LifecycleTransition {
            host_action,
            domain_mutation,
        } = transition;
        let compensation = host_action.compensation();
        let closure_ctx = ctx.clone();
        self.execute_lifecycle_change(ctx, server_id, move |server, service| {
            Box::pin(async move {
                let mut updated_server = server.clone();
                domain_mutation(&mut updated_server, &*service.clock)?;
                let stderr = host_action
                    .execute(&closure_ctx, &*service.host, server)
                    .await?;
                Ok(
                    LifecycleChange::with_compensation(updated_server, compensation)
                        .with_startup_stderr(stderr),
                )
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
        ctx: &RequestContext,
        request: RegisterMcpServerRequest,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        let server_name = McpServerName::new(request.name)?;
        let registration = McpServerRegistration::new(server_name, request.transport, &*self.clock);
        self.repository.register(ctx, &registration).await?;
        Ok(registration)
    }

    /// Starts a registered MCP server.
    ///
    /// Returns a [`LifecycleStartResult`] containing the updated server
    /// registration and any startup stderr captured from the host.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerLifecycleServiceError::NotFound`] when no server has
    /// the given ID, domain errors for invalid lifecycle transitions, or host
    /// and persistence errors.
    pub async fn start(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<LifecycleStartResult> {
        let transition = LifecycleTransition::new(
            LifecycleHostAction::Start,
            |server: &mut McpServerRegistration, clock: &C| {
                server.mark_started(McpServerHealthSnapshot::unknown(clock.utc()), clock)
            },
        );
        let change = self
            .execute_transition_with_compensation(ctx, server_id, transition)
            .await?;
        let startup_stderr = change.startup_stderr;
        let server = self.refresh_health(ctx, server_id).await?;
        Ok(LifecycleStartResult {
            server,
            startup_stderr,
        })
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
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        let transition = LifecycleTransition::new(
            LifecycleHostAction::Stop,
            |server: &mut McpServerRegistration, clock: &C| server.mark_stopped(clock),
        );
        Ok(self
            .execute_transition_with_compensation(ctx, server_id, transition)
            .await?
            .updated_server)
    }

    /// Refreshes and persists server health.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerLifecycleServiceError::NotFound`] when no server
    /// has the given ID, or host and persistence errors.
    pub async fn refresh_health(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<McpServerRegistration> {
        let closure_ctx = ctx.clone();
        Ok(self
            .execute_lifecycle_change(ctx, server_id, |server, service| {
                Box::pin(async move {
                    let health_snapshot = service.host.health(&closure_ctx, server).await?;
                    let mut refreshed_server = server.clone();
                    refreshed_server.update_health(health_snapshot, &*service.clock);
                    Ok(LifecycleChange::without_compensation(refreshed_server))
                })
            })
            .await?
            .updated_server)
    }

    /// Lists all registered MCP servers.
    ///
    /// # Errors
    ///
    /// Returns persistence-layer errors from the repository.
    pub async fn list_all(
        &self,
        ctx: &RequestContext,
    ) -> McpServerLifecycleServiceResult<Vec<McpServerRegistration>> {
        Ok(self.repository.list_all(ctx).await?)
    }

    /// Finds a registered server by name.
    ///
    /// # Errors
    ///
    /// Returns domain validation errors when the name is invalid and
    /// persistence errors from the repository.
    pub async fn find_by_name(
        &self,
        ctx: &RequestContext,
        server_name: &str,
    ) -> McpServerLifecycleServiceResult<Option<McpServerRegistration>> {
        let validated_name = McpServerName::new(server_name)?;
        Ok(self.repository.find_by_name(ctx, &validated_name).await?)
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
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> McpServerLifecycleServiceResult<Vec<McpToolDefinition>> {
        let server = self.find_server_or_error(ctx, server_id).await?;
        server.ensure_can_query_tools()?;
        Ok(self.host.list_tools(ctx, &server).await?)
    }
}

#[cfg(test)]
mod tests;
