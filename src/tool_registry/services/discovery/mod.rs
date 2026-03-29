//! Service layer for tool discovery, catalog management, and call routing.
//!
//! [`ToolDiscoveryRoutingService`] orchestrates tool discovery, schema
//! validation, policy enforcement, call routing, stderr capture, and
//! audit recording.

use crate::context::RequestContext;
use crate::tool_registry::{
    domain::{
        CatalogEntry, LogRetentionPolicy, McpServerId, ToolCallRequest, ToolCallResult,
        ToolGovernanceDecision, ToolRegistryDomainError, validation::validate_parameters,
    },
    ports::{
        CompletedToolCall, McpServerHost, McpServerHostError, McpServerRegistryError,
        McpServerRegistryRepository, ToolCatalogError, ToolCatalogRepository,
        ToolExecutionGovernance, ToolGovernanceError, ToolLogStore, ToolLogStoreError,
    },
};
use mockable::Clock;
use std::sync::Arc;

mod log_and_audit;
mod types;

pub use types::{
    ServicePorts, ToolDiscoveryRoutingServiceError, ToolDiscoveryRoutingServiceResult,
};

/// Tool discovery, catalog management, and call routing service.
///
/// This service is a sibling to [`super::McpServerLifecycleService`],
/// managing tool catalog persistence and call routing as distinct
/// responsibilities from server lifecycle state transitions.
pub struct ToolDiscoveryRoutingService<Cat, Reg, H, Gov, Log, C>
where
    Cat: ToolCatalogRepository,
    Reg: McpServerRegistryRepository,
    H: McpServerHost,
    Gov: ToolExecutionGovernance,
    Log: ToolLogStore,
    C: Clock + Send + Sync,
{
    catalog: Arc<Cat>,
    registry: Arc<Reg>,
    host: Arc<H>,
    governance: Arc<Gov>,
    log_store: Arc<Log>,
    retention_policy: LogRetentionPolicy,
    clock: Arc<C>,
}

impl<Cat, Reg, H, Gov, Log, C> ToolDiscoveryRoutingService<Cat, Reg, H, Gov, Log, C>
where
    Cat: ToolCatalogRepository,
    Reg: McpServerRegistryRepository,
    H: McpServerHost,
    Gov: ToolExecutionGovernance,
    Log: ToolLogStore,
    C: Clock + Send + Sync,
{
    /// Creates a new discovery and routing service.
    #[must_use]
    pub fn new(
        ports: ServicePorts<Cat, Reg, H, Gov, Log>,
        retention_policy: LogRetentionPolicy,
        clock: Arc<C>,
    ) -> Self {
        Self {
            catalog: ports.catalog,
            registry: ports.registry,
            host: ports.host,
            governance: ports.governance,
            log_store: ports.log_store,
            retention_policy,
            clock,
        }
    }

    /// Discovers tools from a running server and persists them in the catalog.
    ///
    /// # Errors
    ///
    /// Returns an error when the server is missing, not running, or the
    /// catalog cannot be updated.
    pub async fn discover_and_persist_tools(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<Vec<CatalogEntry>> {
        let server = self
            .registry
            .find_by_id(ctx, server_id)
            .await?
            .ok_or(ToolDiscoveryRoutingServiceError::NotFound(server_id))?;
        server.ensure_can_query_tools()?;

        let tools = self.host.list_tools(ctx, &server).await?;
        let entries: Vec<CatalogEntry> = tools
            .into_iter()
            .map(|tool| CatalogEntry::new(server_id, server.name().clone(), tool, &*self.clock))
            .collect();
        self.catalog
            .sync_server_tools(ctx, server_id, &entries)
            .await
            .map_err(|err| match err {
                ToolCatalogError::DuplicateEntry {
                    tool_name,
                    server_count,
                    ..
                } => ToolDiscoveryRoutingServiceError::Domain(
                    ToolRegistryDomainError::AmbiguousToolName {
                        tool_name,
                        server_count,
                    },
                ),
                other => ToolDiscoveryRoutingServiceError::Catalog(other),
            })?;
        Ok(entries)
    }

    /// Marks all tools for a server as unavailable in the catalog.
    ///
    /// # Errors
    ///
    /// Returns an error when catalog persistence fails.
    pub async fn mark_tools_unavailable(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<()> {
        self.set_tools_availability(ctx, server_id, false).await
    }

    /// Marks all tools for a server as available in the catalog.
    ///
    /// # Errors
    ///
    /// Returns an error when catalog persistence fails.
    pub async fn mark_tools_available(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<()> {
        self.set_tools_availability(ctx, server_id, true).await
    }

    /// Returns the complete tool catalog.
    ///
    /// # Errors
    ///
    /// Returns an error when catalog lookup fails.
    pub async fn list_catalog(
        &self,
        ctx: &RequestContext,
    ) -> ToolDiscoveryRoutingServiceResult<Vec<CatalogEntry>> {
        Ok(self.catalog.list_all(ctx).await?)
    }

    /// Routes a tool call through validation, governance, execution, stderr
    /// capture, and audit recording.
    ///
    /// # Errors
    ///
    /// Returns an error when server resolution or validation fails, when
    /// governance denies or errors before execution, or when host execution
    /// fails.
    ///
    /// Post-call audit persistence and governance observation failures are
    /// awaited for side effects but are not propagated from this method.
    pub async fn call_tool(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
    ) -> ToolDiscoveryRoutingServiceResult<ToolCallResult> {
        let entry = match self.resolve_and_validate(ctx, request).await {
            Ok(entry) => entry,
            Err((maybe_entry, err)) => {
                if let Some(entry) = maybe_entry {
                    let rejected = log_and_audit::RejectedCallContext {
                        request,
                        server_id: entry.server_id(),
                    };
                    self.audit_rejection(ctx, &rejected, &err).await;
                }
                return Err(err);
            }
        };

        self.execute_and_audit(ctx, request, &entry).await
    }

    async fn resolve_and_validate(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
    ) -> Result<CatalogEntry, (Option<CatalogEntry>, ToolDiscoveryRoutingServiceError)> {
        let entries = match self
            .catalog
            .find_by_tool_name(ctx, request.tool_name())
            .await
        {
            Ok(e) => e,
            Err(err) => return Err((None, err.into())),
        };

        let entry = match entries.len() {
            0 => {
                let err = ToolRegistryDomainError::ToolNotFound(request.tool_name().to_owned());
                return Err((None, err.into()));
            }
            1 => {
                let Some(entry) = entries.into_iter().next() else {
                    let err = ToolRegistryDomainError::ToolNotFound(request.tool_name().to_owned());
                    return Err((None, err.into()));
                };
                entry
            }
            n => {
                let err = ToolRegistryDomainError::AmbiguousToolName {
                    tool_name: request.tool_name().to_owned(),
                    server_count: n,
                };
                return Err((None, err.into()));
            }
        };

        if let Err(err) = self.validate_entry(ctx, &entry, request).await {
            return Err((Some(entry), err));
        }
        Ok(entry)
    }

    async fn set_tools_availability(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        available: bool,
    ) -> ToolDiscoveryRoutingServiceResult<()> {
        let res = if available {
            self.catalog
                .mark_server_tools_available(ctx, server_id)
                .await
        } else {
            self.catalog
                .mark_server_tools_unavailable(ctx, server_id)
                .await
        };
        res.map_err(ToolDiscoveryRoutingServiceError::Catalog)
    }

    async fn validate_entry(
        &self,
        ctx: &RequestContext,
        entry: &CatalogEntry,
        request: &ToolCallRequest,
    ) -> ToolDiscoveryRoutingServiceResult<()> {
        if !entry.available() {
            return Err(ToolRegistryDomainError::ToolUnavailable {
                tool_name: request.tool_name().to_owned(),
                server_id: entry.server_id(),
            }
            .into());
        }
        validate_parameters(entry.tool().input_schema(), request.parameters()).map_err(|err| {
            match err {
                ToolRegistryDomainError::SchemaValidationFailed { reason, .. } => {
                    ToolRegistryDomainError::SchemaValidationFailed {
                        tool_name: request.tool_name().to_owned(),
                        reason,
                    }
                }
                other => other,
            }
        })?;
        let decision = self
            .governance
            .enforce_before_call(ctx, request, entry)
            .await?;
        if let ToolGovernanceDecision::Deny { reason } = decision {
            return Err(ToolRegistryDomainError::PolicyDenied {
                tool_name: request.tool_name().to_owned(),
                reason,
            }
            .into());
        }
        Ok(())
    }

    async fn execute_and_audit(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
    ) -> ToolDiscoveryRoutingServiceResult<ToolCallResult> {
        let server =
            log_and_audit::find_running_server_or_audit_rejection(self, ctx, request, entry)
                .await?;
        let execute_result = self.host.call_tool(ctx, &server, request).await;

        let completed_at = self.clock.utc();
        let (result, stderr_output, host_error) = log_and_audit::build_tool_call_result(
            request,
            execute_result,
            entry.server_id(),
            completed_at,
        );
        let completed = log_and_audit::CompletedCallContext {
            request,
            result: &result,
        };
        self.capture_and_audit(ctx, &completed, stderr_output).await;
        if let Err(err) = self
            .governance
            .observe_after_call(
                ctx,
                &CompletedToolCall {
                    request,
                    entry,
                    result: &result,
                },
            )
            .await
        {
            log_and_audit::warn_post_call_observation_failure(request, entry, &err);
        }

        log_and_audit::return_result_or_host_error(result, host_error)
    }

    async fn find_running_server(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<crate::tool_registry::domain::McpServerRegistration>
    {
        let server = self
            .registry
            .find_by_id(ctx, server_id)
            .await?
            .ok_or(ToolDiscoveryRoutingServiceError::NotFound(server_id))?;
        server.ensure_can_query_tools()?;
        Ok(server)
    }
}

#[cfg(test)]
mod governance_tests;
#[cfg(test)]
mod tests;
