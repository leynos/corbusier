//! Log capture and audit recording methods for the discovery service.

use crate::context::RequestContext;
use crate::tool_registry::{
    domain::{
        LogCaptureContext, LogEntryMetadata, McpServerId, ToolCallAuditRecord, ToolCallRequest,
        ToolCallResult,
    },
    ports::{
        McpServerHost, McpServerRegistryRepository, StoreLogRequest, SweepContext,
        ToolCatalogRepository, ToolExecutionGovernance, ToolLogStore,
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
