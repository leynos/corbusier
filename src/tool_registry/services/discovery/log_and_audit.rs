//! Log capture and audit recording methods for the discovery service.

use crate::context::RequestContext;
use crate::tool_registry::{
    domain::{
        LogCaptureContext, LogEntryMetadata, McpServerId, ToolCallAuditRecord, ToolCallId,
        ToolCallRequest, ToolCallResult,
    },
    ports::{
        McpServerHost, McpServerRegistryRepository, SweepContext, ToolCatalogRepository,
        ToolLogStore, ToolPolicyEnforcer,
    },
};
use mockable::Clock;

use super::{
    ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError,
    ToolDiscoveryRoutingServiceResult,
};

impl<Cat, Reg, H, Pol, Log, C> ToolDiscoveryRoutingService<Cat, Reg, H, Pol, Log, C>
where
    Cat: ToolCatalogRepository,
    Reg: McpServerRegistryRepository,
    H: McpServerHost,
    Pol: ToolPolicyEnforcer,
    Log: ToolLogStore,
    C: Clock + Send + Sync,
{
    /// Best-effort audit recording for a pre-execution rejection.
    #[expect(
        clippy::too_many_arguments,
        reason = "RequestContext plumbing adds one parameter beyond the natural arity"
    )]
    pub(super) async fn audit_rejection(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        server_id: McpServerId,
        err: &ToolDiscoveryRoutingServiceError,
    ) {
        let audit = ToolCallAuditRecord::for_rejection(request, server_id, err, self.clock.utc());
        let _audit_result = self.catalog.record_audit(ctx, &audit).await;
    }

    /// Best-effort stderr capture and audit recording for a completed
    /// call.
    #[expect(
        clippy::too_many_arguments,
        reason = "RequestContext plumbing adds one parameter beyond the natural arity"
    )]
    pub(super) async fn capture_and_audit(
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
        self.log_store
            .store_log(ctx, &metadata, stderr, &self.retention_policy)
            .await?;
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
            entry_metadata: &[],
        };
        Ok(self.log_store.sweep_expired(ctx, server_id, &sweep).await?)
    }

    /// Best-effort stderr capture for a tool call.  Returns the object
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
            tenant_id: ctx.tenant_id(),
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
