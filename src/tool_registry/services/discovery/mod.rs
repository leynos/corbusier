//! Service layer for tool discovery, catalog management, and call routing.
//!
//! [`ToolDiscoveryRoutingService`] orchestrates tool discovery, schema
//! validation, policy enforcement, call routing, stderr capture, and
//! audit recording.

use crate::context::RequestContext;
use crate::tool_registry::{
    domain::{
        CatalogEntry, LogCaptureContext, LogEntryMetadata, LogRetentionPolicy, McpServerId,
        PolicyDecision, ToolCallAuditRecord, ToolCallId, ToolCallOutcome, ToolCallRequest,
        ToolCallResult, ToolCallTiming, ToolRegistryDomainError, validation::validate_parameters,
    },
    ports::{
        McpServerHost, McpServerHostError, McpServerRegistryError, McpServerRegistryRepository,
        SweepContext, ToolCatalogError, ToolCatalogRepository, ToolLogStore, ToolLogStoreError,
        ToolPolicyEnforcer, ToolPolicyError,
    },
};
use mockable::Clock;
use std::sync::Arc;
use thiserror::Error;

/// Service-level errors for tool discovery and routing operations.
#[derive(Debug, Error)]
pub enum ToolDiscoveryRoutingServiceError {
    /// Domain validation failed.
    #[error(transparent)]
    Domain(#[from] ToolRegistryDomainError),
    /// Catalog persistence failed.
    #[error(transparent)]
    Catalog(#[from] ToolCatalogError),
    /// Registry operation failed.
    #[error(transparent)]
    Registry(#[from] McpServerRegistryError),
    /// Host operation failed.
    #[error(transparent)]
    Host(#[from] McpServerHostError),
    /// Policy evaluation failed.
    #[error(transparent)]
    Policy(#[from] ToolPolicyError),
    /// Log store operation failed.
    #[error(transparent)]
    LogStore(#[from] ToolLogStoreError),
    /// No server exists with the given identifier.
    #[error("MCP server {0} not found")]
    NotFound(McpServerId),
}

/// Result type for discovery and routing service operations.
pub type ToolDiscoveryRoutingServiceResult<T> = Result<T, ToolDiscoveryRoutingServiceError>;

/// Port dependencies for [`ToolDiscoveryRoutingService`].
pub struct ServicePorts<Cat, Reg, H, Pol, Log> {
    /// Catalog repository.
    pub catalog: Arc<Cat>,
    /// Server registry.
    pub registry: Arc<Reg>,
    /// Server host.
    pub host: Arc<H>,
    /// Policy enforcer.
    pub policy: Arc<Pol>,
    /// Log store.
    pub log_store: Arc<Log>,
}

/// Tool discovery, catalog management, and call routing service.
///
/// This service is a sibling to [`super::McpServerLifecycleService`],
/// managing tool catalog persistence and call routing as distinct
/// responsibilities from server lifecycle state transitions.
pub struct ToolDiscoveryRoutingService<Cat, Reg, H, Pol, Log, C>
where
    Cat: ToolCatalogRepository,
    Reg: McpServerRegistryRepository,
    H: McpServerHost,
    Pol: ToolPolicyEnforcer,
    Log: ToolLogStore,
    C: Clock + Send + Sync,
{
    catalog: Arc<Cat>,
    registry: Arc<Reg>,
    host: Arc<H>,
    policy: Arc<Pol>,
    log_store: Arc<Log>,
    retention_policy: LogRetentionPolicy,
    clock: Arc<C>,
}

impl<Cat, Reg, H, Pol, Log, C> ToolDiscoveryRoutingService<Cat, Reg, H, Pol, Log, C>
where
    Cat: ToolCatalogRepository,
    Reg: McpServerRegistryRepository,
    H: McpServerHost,
    Pol: ToolPolicyEnforcer,
    Log: ToolLogStore,
    C: Clock + Send + Sync,
{
    /// Creates a new discovery and routing service.
    #[must_use]
    pub fn new(
        ports: ServicePorts<Cat, Reg, H, Pol, Log>,
        retention_policy: LogRetentionPolicy,
        clock: Arc<C>,
    ) -> Self {
        Self {
            catalog: ports.catalog,
            registry: ports.registry,
            host: ports.host,
            policy: ports.policy,
            log_store: ports.log_store,
            retention_policy,
            clock,
        }
    }

    /// Discovers tools from a running server and persists them in the
    /// catalog.
    ///
    /// # Errors
    ///
    /// Returns errors when the server is not found, not running, or
    /// catalog persistence fails.
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
            .await?;
        Ok(entries)
    }

    /// Marks all tools for a server as unavailable in the catalog.
    ///
    /// # Errors
    /// Returns [`ToolCatalogError`] on persistence failures.
    pub async fn mark_tools_unavailable(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<()> {
        self.catalog
            .mark_server_tools_unavailable(ctx, server_id)
            .await?;
        Ok(())
    }

    /// Marks all tools for a server as available in the catalog.
    ///
    /// # Errors
    /// Returns [`ToolCatalogError`] on persistence failures.
    pub async fn mark_tools_available(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<()> {
        self.catalog
            .mark_server_tools_available(ctx, server_id)
            .await?;
        Ok(())
    }

    /// Returns the complete tool catalog.
    ///
    /// # Errors
    /// Returns [`ToolCatalogError`] on persistence failures.
    pub async fn list_catalog(
        &self,
        ctx: &RequestContext,
    ) -> ToolDiscoveryRoutingServiceResult<Vec<CatalogEntry>> {
        Ok(self.catalog.list_all(ctx).await?)
    }

    /// Routes a tool call through validation, policy, execution, stderr
    /// capture, and audit recording.
    ///
    /// Pre-execution rejections (unavailable tool, schema validation
    /// failure, policy denial) are audited as failures before the error
    /// propagates. Only `ToolNotFound` skips auditing because no
    /// `server_id` is available.
    ///
    /// # Errors
    /// Returns errors for tool resolution, schema validation, policy
    /// denial, host execution failures, or timeout.
    pub async fn call_tool(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
    ) -> ToolDiscoveryRoutingServiceResult<ToolCallResult> {
        let entry = match self.resolve_and_validate(ctx, request).await {
            Ok(entry) => entry,
            Err((maybe_entry, err)) => {
                if let Some(entry) = maybe_entry {
                    self.audit_rejection(ctx, request, entry.server_id(), &err)
                        .await;
                }
                return Err(err);
            }
        };

        self.execute_and_audit(ctx, request, &entry).await
    }

    /// Resolves a tool from the catalog, checks availability, validates
    /// parameters, and enforces policy. On failure returns the catalog
    /// entry (if resolved) alongside the error for audit purposes.
    async fn resolve_and_validate(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
    ) -> Result<CatalogEntry, (Option<CatalogEntry>, ToolDiscoveryRoutingServiceError)> {
        let entry = match self
            .catalog
            .find_by_tool_name(ctx, request.tool_name())
            .await
        {
            Ok(Some(e)) => e,
            Ok(None) => {
                let err = ToolRegistryDomainError::ToolNotFound(request.tool_name().to_owned());
                return Err((None, err.into()));
            }
            Err(err) => return Err((None, err.into())),
        };

        if let Err(err) = self.validate_entry(ctx, &entry, request).await {
            return Err((Some(entry), err));
        }
        Ok(entry)
    }

    /// Validates availability, schema, and policy for a resolved entry.
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
            .policy
            .evaluate(ctx, request.tool_name(), request.parameters())
            .await?;
        if let PolicyDecision::Deny { reason } = decision {
            return Err(ToolRegistryDomainError::PolicyDenied {
                tool_name: request.tool_name().to_owned(),
                reason,
            }
            .into());
        }
        Ok(())
    }

    /// Executes a validated tool call and records the audit trail.
    async fn execute_and_audit(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
    ) -> ToolDiscoveryRoutingServiceResult<ToolCallResult> {
        let server = self.find_running_server(ctx, entry.server_id()).await?;
        let execute_result = self
            .host
            .call_tool(ctx, &server, request.tool_name(), request.parameters())
            .await;

        let completed_at = self.clock.utc();
        let duration = (completed_at - request.initiated_at())
            .to_std()
            .unwrap_or_default();
        let (outcome, stderr_output, host_error) = match execute_result {
            Ok(r) => (
                ToolCallOutcome::Success { content: r.content },
                r.stderr_output,
                None,
            ),
            Err(e) => (
                ToolCallOutcome::Failure {
                    error: e.to_string(),
                },
                None,
                Some(e),
            ),
        };

        let timing = ToolCallTiming {
            duration,
            completed_at,
        };
        let result = ToolCallResult::from_request(request, entry.server_id(), outcome, timing);
        self.capture_and_audit(ctx, request, &result, stderr_output)
            .await;

        if let Some(host_err) = host_error {
            return Err(host_err.into());
        }
        Ok(result)
    }

    /// Loads a server from the registry and verifies it is running.
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

    /// Best-effort audit recording for a pre-execution rejection.
    #[expect(
        clippy::too_many_arguments,
        reason = "RequestContext plumbing adds one parameter beyond the natural arity"
    )]
    async fn audit_rejection(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        server_id: McpServerId,
        err: &ToolDiscoveryRoutingServiceError,
    ) {
        let audit = ToolCallAuditRecord::for_rejection(request, server_id, err, self.clock.utc());
        let _audit_result = self.catalog.record_audit(ctx, &audit).await;
    }

    /// Best-effort stderr capture and audit recording for a completed call.
    #[expect(
        clippy::too_many_arguments,
        reason = "RequestContext plumbing adds one parameter beyond the natural arity"
    )]
    async fn capture_and_audit(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        result: &ToolCallResult,
        stderr_output: Option<bytes::Bytes>,
    ) {
        let stderr_log_path = self
            .try_capture_tool_call_stderr(ctx, result.server_id(), request.call_id(), stderr_output)
            .await;
        let mut audit = ToolCallAuditRecord::from_result(
            result,
            request.parameters().clone(),
            request.initiated_at(),
        );
        if let Some(path) = &stderr_log_path {
            audit = audit.with_stderr_log_path(path);
        }
        let _audit_result = self.catalog.record_audit(ctx, &audit).await;
    }

    /// Stores startup stderr captured from `McpServerHost::start`.
    /// Also triggers a retention sweep for the server.
    ///
    /// # Errors
    /// Returns [`ToolLogStoreError`] when the store operation fails.
    pub async fn store_startup_stderr(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        stderr: bytes::Bytes,
    ) -> ToolDiscoveryRoutingServiceResult<LogEntryMetadata> {
        let byte_count = stderr.len() as u64;
        let capture_ctx = LogCaptureContext {
            clock: &*self.clock,
            retention: &self.retention_policy,
        };
        let metadata = LogEntryMetadata::for_startup(server_id, byte_count, &capture_ctx);
        self.log_store
            .store_log(ctx, &metadata, stderr, &self.retention_policy)
            .await?;
        let _sweep_count = self.sweep_expired_logs(ctx, server_id).await;
        Ok(metadata)
    }

    /// Triggers a retention sweep for a specific server's logs.
    ///
    /// # Errors
    /// Returns [`ToolLogStoreError`] when the sweep fails.
    pub async fn sweep_expired_logs(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<usize> {
        let now = self.clock.utc();
        let sweep = SweepContext {
            policy: &self.retention_policy,
            now,
            entry_metadata: &[],
        };
        Ok(self.log_store.sweep_expired(ctx, server_id, &sweep).await?)
    }

    /// Best-effort stderr capture for a tool call. Returns the object
    /// store path on success, or `None` on failure.
    #[expect(
        clippy::too_many_arguments,
        reason = "RequestContext plumbing adds one parameter beyond the natural arity"
    )]
    async fn try_capture_tool_call_stderr(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        call_id: ToolCallId,
        stderr_output: Option<bytes::Bytes>,
    ) -> Option<String> {
        let stderr = stderr_output.filter(|b| !b.is_empty())?;
        let byte_count = stderr.len() as u64;
        let capture_ctx = LogCaptureContext {
            clock: &*self.clock,
            retention: &self.retention_policy,
        };
        let metadata =
            LogEntryMetadata::for_tool_call(server_id, call_id, byte_count, &capture_ctx);
        let path = metadata.object_path().to_owned();
        match self
            .log_store
            .store_log(ctx, &metadata, stderr, &self.retention_policy)
            .await
        {
            Ok(()) => Some(path),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
