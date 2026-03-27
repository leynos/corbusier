//! Log capture and audit recording methods for the discovery service.

use crate::context::RequestContext;
use crate::tool_registry::{
    domain::{
        CatalogEntry, LogCaptureContext, LogEntryMetadata, McpServerId, ToolCallAuditRecord,
        ToolCallOutcome, ToolCallRequest, ToolCallResult, ToolCallTiming,
    },
    ports::{
        McpServerHost, McpServerHostError, McpServerRegistryRepository, StoreLogRequest,
        SweepContext, ToolCallHostResult, ToolCatalogRepository, ToolExecutionGovernance,
        ToolLogStore,
    },
};
use mockable::Clock;

use super::{
    ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError,
    ToolDiscoveryRoutingServiceResult,
};

/// Bundled context for a pre-execution rejection audit record.
///
/// Groups `request` and `server_id`, which are always used together
/// when recording a call that was rejected before reaching the host.
pub(super) struct RejectedCallContext<'a> {
    pub request: &'a ToolCallRequest,
    pub server_id: McpServerId,
}

/// Bundled context for a completed-call audit and stderr capture.
///
/// Groups `request` and `result`, which jointly describe the call that
/// was executed and whose outcome is being recorded.
pub(super) struct CompletedCallContext<'a> {
    pub request: &'a ToolCallRequest,
    pub result: &'a ToolCallResult,
}

pub(super) fn build_tool_call_result(
    request: &ToolCallRequest,
    result: Result<ToolCallHostResult, McpServerHostError>,
    server_id: McpServerId,
    completed_at: chrono::DateTime<chrono::Utc>,
) -> (
    ToolCallResult,
    Option<bytes::Bytes>,
    Option<McpServerHostError>,
) {
    let duration = (completed_at - request.initiated_at())
        .to_std()
        .unwrap_or_default();
    let (outcome, stderr_output, host_error) = match result {
        Ok(response) => (
            ToolCallOutcome::Success {
                content: response.content,
            },
            response.stderr_output,
            None,
        ),
        Err(error) => (
            ToolCallOutcome::Failure {
                error: error.to_string(),
            },
            None,
            Some(error),
        ),
    };
    let timing = ToolCallTiming {
        duration,
        completed_at,
    };
    (
        ToolCallResult::from_request(request, server_id, outcome, timing),
        stderr_output,
        host_error,
    )
}

pub(super) async fn find_running_server_or_audit_rejection<Cat, Reg, H, Gov, Log, C>(
    service: &ToolDiscoveryRoutingService<Cat, Reg, H, Gov, Log, C>,
    ctx: &RequestContext,
    request: &ToolCallRequest,
    entry: &CatalogEntry,
) -> ToolDiscoveryRoutingServiceResult<crate::tool_registry::domain::McpServerRegistration>
where
    Cat: ToolCatalogRepository,
    Reg: McpServerRegistryRepository,
    H: McpServerHost,
    Gov: ToolExecutionGovernance,
    Log: ToolLogStore,
    C: Clock + Send + Sync,
{
    match service.find_running_server(ctx, entry.server_id()).await {
        Ok(server) => Ok(server),
        Err(err) => {
            let rejected = RejectedCallContext {
                request,
                server_id: entry.server_id(),
            };
            service.audit_rejection(ctx, &rejected, &err).await;
            Err(err)
        }
    }
}

pub(super) fn warn_post_call_observation_failure(
    request: &ToolCallRequest,
    entry: &CatalogEntry,
    err: &crate::tool_registry::ports::ToolGovernanceError,
) {
    tracing::warn!(
        call_id = %request.call_id(),
        tool_name = request.tool_name(),
        server_id = %entry.server_id(),
        error = %err,
        "post-tool-use governance observation failed after tool execution"
    );
}

pub(super) fn return_result_or_host_error(
    result: ToolCallResult,
    host_error: Option<McpServerHostError>,
) -> ToolDiscoveryRoutingServiceResult<ToolCallResult> {
    if let Some(error) = host_error {
        return Err(error.into());
    }
    Ok(result)
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
    /// Best-effort audit recording for a pre-execution rejection.
    pub(super) async fn audit_rejection(
        &self,
        ctx: &RequestContext,
        call: &RejectedCallContext<'_>,
        err: &ToolDiscoveryRoutingServiceError,
    ) {
        let audit =
            ToolCallAuditRecord::for_rejection(call.request, call.server_id, err, self.clock.utc());
        let _audit_result = self.catalog.record_audit(ctx, &audit).await;
    }

    /// Best-effort stderr capture and audit recording for a completed
    /// call.
    pub(super) async fn capture_and_audit(
        &self,
        ctx: &RequestContext,
        call: &CompletedCallContext<'_>,
        stderr_output: Option<bytes::Bytes>,
    ) {
        let stderr_log_path = self
            .try_capture_tool_call_stderr(ctx, call.result, stderr_output)
            .await;
        let mut audit = ToolCallAuditRecord::from_result(
            call.result,
            call.request.parameters().clone(),
            call.request.initiated_at(),
        );
        if let Some(path) = &stderr_log_path {
            audit = audit.with_stderr_log_path(path);
        }
        let _audit_result = self.catalog.record_audit(ctx, &audit).await;
    }

    /// Stores startup stderr captured from [`McpServerHost::start`].
    /// Also triggers a retention sweep for the server.
    ///
    /// # Errors
    /// Returns [`super::ToolDiscoveryRoutingServiceError`] when the
    /// store operation fails.
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
            tenant_id: ctx.tenant_id(),
        };
        let metadata = LogEntryMetadata::for_startup(server_id, byte_count, &capture_ctx);
        let request = StoreLogRequest {
            metadata: &metadata,
            content: stderr,
            retention: &self.retention_policy,
        };
        self.log_store.store_log(ctx, &request).await?;
        let _sweep_count = self.sweep_expired_logs(ctx, server_id).await;
        Ok(metadata)
    }

    /// Triggers a retention sweep for a specific server's logs.
    ///
    /// # Errors
    /// Returns [`super::ToolDiscoveryRoutingServiceError`] when the
    /// sweep fails.
    pub async fn sweep_expired_logs(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolDiscoveryRoutingServiceResult<usize> {
        let now = self.clock.utc();
        let sweep = SweepContext {
            policy: &self.retention_policy,
            now,
        };
        Ok(self.log_store.sweep_expired(ctx, server_id, &sweep).await?)
    }

    /// Best-effort stderr capture for a tool call.  Returns the object
    /// store path on success, or `None` on failure.
    async fn try_capture_tool_call_stderr(
        &self,
        ctx: &RequestContext,
        result: &ToolCallResult,
        stderr_output: Option<bytes::Bytes>,
    ) -> Option<String> {
        let stderr = stderr_output.filter(|b| !b.is_empty())?;
        let byte_count = stderr.len() as u64;
        let capture_ctx = LogCaptureContext {
            clock: &*self.clock,
            retention: &self.retention_policy,
            tenant_id: ctx.tenant_id(),
        };
        let metadata = LogEntryMetadata::for_tool_call(
            result.server_id(),
            result.call_id(),
            byte_count,
            &capture_ctx,
        );
        let path = metadata.object_path().to_owned();
        let request = StoreLogRequest {
            metadata: &metadata,
            content: stderr,
            retention: &self.retention_policy,
        };
        match self.log_store.store_log(ctx, &request).await {
            Ok(()) => Some(path),
            Err(_) => None,
        }
    }
}
