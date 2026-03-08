//! Service layer for tool discovery, catalog management, and call routing.
//!
//! [`ToolDiscoveryRoutingService`] orchestrates tool discovery, schema validation, policy
//! enforcement, call routing, stderr capture, and audit recording.

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
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<Vec<CatalogEntry>> {
        let server = self
            .registry
            .find_by_id(server_id)
            .await?
            .ok_or(ToolDiscoveryRoutingServiceError::NotFound(server_id))?;
        server.ensure_can_query_tools()?;

        let tools = self.host.list_tools(&server).await?;
        let entries: Vec<CatalogEntry> = tools
            .into_iter()
            .map(|tool| CatalogEntry::new(server_id, server.name().clone(), tool, &*self.clock))
            .collect();

        self.catalog.sync_server_tools(server_id, &entries).await?;
        Ok(entries)
    }

    /// Marks all tools for a server as unavailable in the catalog.
    ///
    /// # Errors
    /// Returns [`ToolCatalogError`] on persistence failures.
    pub async fn mark_tools_unavailable(
        &self,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<()> {
        self.catalog
            .mark_server_tools_unavailable(server_id)
            .await?;
        Ok(())
    }

    /// Marks all tools for a server as available in the catalog.
    ///
    /// # Errors
    /// Returns [`ToolCatalogError`] on persistence failures.
    pub async fn mark_tools_available(
        &self,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<()> {
        self.catalog.mark_server_tools_available(server_id).await?;
        Ok(())
    }

    /// Returns the complete tool catalog.
    ///
    /// # Errors
    /// Returns [`ToolCatalogError`] on persistence failures.
    pub async fn list_catalog(&self) -> ToolDiscoveryRoutingServiceResult<Vec<CatalogEntry>> {
        Ok(self.catalog.list_all().await?)
    }

    /// Routes a tool call through validation, policy, execution, stderr
    /// capture, and audit recording.
    ///
    /// # Errors
    /// Returns errors for tool resolution, schema validation, policy
    /// denial, host execution failures, or timeout.
    pub async fn call_tool(
        &self,
        request: &ToolCallRequest,
    ) -> ToolDiscoveryRoutingServiceResult<ToolCallResult> {
        let entry = self.resolve_and_validate(request).await?;

        // Execute via host.
        let execute_result = self
            .host
            .call_tool(
                &self.find_running_server(entry.server_id()).await?,
                request.tool_name(),
                request.parameters().clone(),
            )
            .await;

        let completed_at = self.clock.utc();
        let duration = (completed_at - request.initiated_at())
            .to_std()
            .unwrap_or_default();

        let (outcome, stderr_output, host_error) = match execute_result {
            Ok(host_result) => (
                ToolCallOutcome::Success {
                    content: host_result.content,
                },
                host_result.stderr_output,
                None,
            ),
            Err(host_err) => {
                let outcome = ToolCallOutcome::Failure {
                    error: host_err.to_string(),
                };
                (outcome, None, Some(host_err))
            }
        };

        let timing = ToolCallTiming {
            duration,
            completed_at,
        };
        let result = ToolCallResult::from_request(request, entry.server_id(), outcome, timing);

        self.capture_and_audit(request, &result, stderr_output)
            .await;

        if let Some(host_err) = host_error {
            return Err(host_err.into());
        }
        Ok(result)
    }

    /// Resolves a tool from the catalog, checks availability, validates
    /// parameters, and enforces policy.
    async fn resolve_and_validate(
        &self,
        request: &ToolCallRequest,
    ) -> ToolDiscoveryRoutingServiceResult<CatalogEntry> {
        let entry = self
            .catalog
            .find_by_tool_name(request.tool_name())
            .await?
            .ok_or_else(|| ToolRegistryDomainError::ToolNotFound(request.tool_name().to_owned()))?;

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
            .evaluate(request.tool_name(), request.parameters())
            .await?;
        if let PolicyDecision::Deny { reason } = decision {
            return Err(ToolRegistryDomainError::PolicyDenied {
                tool_name: request.tool_name().to_owned(),
                reason,
            }
            .into());
        }

        Ok(entry)
    }

    /// Loads a server from the registry and verifies it is running.
    async fn find_running_server(
        &self,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<crate::tool_registry::domain::McpServerRegistration>
    {
        let server = self
            .registry
            .find_by_id(server_id)
            .await?
            .ok_or(ToolDiscoveryRoutingServiceError::NotFound(server_id))?;
        server.ensure_can_query_tools()?;
        Ok(server)
    }

    /// Best-effort stderr capture and audit recording for a completed
    /// tool call.
    async fn capture_and_audit(
        &self,
        request: &ToolCallRequest,
        result: &ToolCallResult,
        stderr_output: Option<bytes::Bytes>,
    ) {
        let stderr_log_path = self
            .try_capture_tool_call_stderr(result.server_id(), request.call_id(), stderr_output)
            .await;

        let mut audit = ToolCallAuditRecord::from_result(
            result,
            request.parameters().clone(),
            request.initiated_at(),
        );
        if let Some(path) = &stderr_log_path {
            audit = audit.with_stderr_log_path(path);
        }
        let _audit_result = self.catalog.record_audit(&audit).await;
    }

    /// Stores startup stderr captured from `McpServerHost::start`.
    /// Also triggers a retention sweep for the server.
    ///
    /// # Errors
    /// Returns [`ToolLogStoreError`] when the store operation fails.
    pub async fn store_startup_stderr(
        &self,
        server_id: McpServerId,
        stderr: bytes::Bytes,
    ) -> ToolDiscoveryRoutingServiceResult<LogEntryMetadata> {
        let byte_count = stderr.len() as u64;
        let ctx = LogCaptureContext {
            clock: &*self.clock,
            retention: &self.retention_policy,
        };
        let metadata = LogEntryMetadata::for_startup(server_id, byte_count, &ctx);
        self.log_store
            .store_log(&metadata, stderr, &self.retention_policy)
            .await?;

        // Best-effort sweep.
        let _sweep_count = self.sweep_expired_logs(server_id).await;

        Ok(metadata)
    }

    /// Triggers a retention sweep for a specific server's logs.
    ///
    /// # Errors
    ///
    /// Returns [`ToolLogStoreError`] when the sweep fails.
    pub async fn sweep_expired_logs(
        &self,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<usize> {
        let now = self.clock.utc();
        // The sweep uses an empty metadata slice since the log store
        // adapter manages its own listing internally for path-based
        // sweeps. Full metadata-based sweeps are used by the Postgres
        // integration where metadata is maintained externally.
        let ctx = SweepContext {
            policy: &self.retention_policy,
            now,
            entry_metadata: &[],
        };
        Ok(self.log_store.sweep_expired(server_id, &ctx).await?)
    }

    /// Best-effort stderr capture for a tool call. Returns the object
    /// store path on success, or `None` on failure.
    async fn try_capture_tool_call_stderr(
        &self,
        server_id: McpServerId,
        call_id: ToolCallId,
        stderr_output: Option<bytes::Bytes>,
    ) -> Option<String> {
        let stderr = stderr_output.filter(|b| !b.is_empty())?;
        let byte_count = stderr.len() as u64;
        let ctx = LogCaptureContext {
            clock: &*self.clock,
            retention: &self.retention_policy,
        };
        let metadata = LogEntryMetadata::for_tool_call(server_id, call_id, byte_count, &ctx);
        let path = metadata.object_path().to_owned();
        match self
            .log_store
            .store_log(&metadata, stderr, &self.retention_policy)
            .await
        {
            Ok(()) => Some(path),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
