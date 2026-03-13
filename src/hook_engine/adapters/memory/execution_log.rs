//! In-memory hook execution log repository.

use crate::context::{RequestContext, TenantId};
use crate::hook_engine::domain::{HookExecutionResult, TriggerContextId};
use crate::hook_engine::ports::{
    HookExecutionLogError, HookExecutionLogRepository, HookExecutionLogResult,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe in-memory hook execution log repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryHookExecutionLogRepository {
    executions: Arc<RwLock<HashMap<TenantId, Vec<HookExecutionResult>>>>,
}

impl InMemoryHookExecutionLogRepository {
    /// Creates an empty in-memory repository.
    ///
    /// Example: `InMemoryHookExecutionLogRepository::new()` creates a log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all stored execution results.
    ///
    /// Example: `repo.all(&ctx)` returns the stored executions.
    ///
    /// # Errors
    ///
    /// Returns [`HookExecutionLogError`] if the lock cannot be acquired.
    pub fn all(&self, ctx: &RequestContext) -> HookExecutionLogResult<Vec<HookExecutionResult>> {
        let executions = self.executions.try_read().map_err(|err| {
            HookExecutionLogError::persistence_failed(std::io::Error::other(format!(
                "hook execution log lock unavailable: {err}"
            )))
        })?;
        Ok(executions
            .get(&ctx.tenant_id())
            .cloned()
            .unwrap_or_default())
    }
}

#[async_trait]
impl HookExecutionLogRepository for InMemoryHookExecutionLogRepository {
    async fn store(
        &self,
        ctx: &RequestContext,
        result: &HookExecutionResult,
    ) -> HookExecutionLogResult<()> {
        let mut executions = self.executions.write().await;
        executions
            .entry(ctx.tenant_id())
            .or_default()
            .push(result.clone());
        Ok(())
    }

    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookExecutionLogResult<Vec<HookExecutionResult>> {
        let executions = self.executions.read().await;
        let tenant_executions = executions
            .get(&ctx.tenant_id())
            .map_or(&[][..], Vec::as_slice);
        let mut filtered: Vec<_> = tenant_executions
            .iter()
            .filter(|result| result.trigger_context_id() == trigger_context_id)
            .cloned()
            .collect();
        filtered.sort_by(|left, right| {
            left.executed_at()
                .cmp(&right.executed_at())
                .then_with(|| left.hook_id().cmp(right.hook_id()))
        });
        Ok(filtered)
    }
}
