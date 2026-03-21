//! In-memory hook execution log repository.

use crate::context::{RequestContext, TenantId};
use crate::hook_engine::domain::{
    HookExecutionPersisted, HookExecutionResult, HookExecutionStatus, TriggerContextId,
};
use crate::hook_engine::ports::{
    HookExecutionLogError, HookExecutionLogRepository, HookExecutionLogResult,
    PendingExecutionRecord, PendingExecutionReservation,
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
    async fn store_pending(
        &self,
        ctx: &RequestContext,
        record: PendingExecutionRecord,
    ) -> HookExecutionLogResult<PendingExecutionReservation> {
        let mut executions = self.executions.write().await;
        let tenant_executions = executions.entry(ctx.tenant_id()).or_default();
        if let Some(existing) = tenant_executions.iter().find(|result| {
            result.trigger_context_id() == record.trigger_context_id
                && result.hook_id() == &record.hook_id
        }) {
            return Ok(PendingExecutionReservation::AlreadyExists(
                existing.execution_id(),
            ));
        }
        tenant_executions.push(HookExecutionResult::from_persisted(
            HookExecutionPersisted {
                execution_id: record.execution_id,
                hook_id: record.hook_id,
                trigger_context_id: record.trigger_context_id,
                trigger_type: record.trigger_type,
                predicate_data: serde_json::Value::Object(serde_json::Map::new()),
                action_results: Vec::new(),
                status: HookExecutionStatus::Pending,
                executed_at: record.executed_at,
            },
        ));
        Ok(PendingExecutionReservation::Created(record.execution_id))
    }

    async fn update_result(
        &self,
        ctx: &RequestContext,
        result: &HookExecutionResult,
    ) -> HookExecutionLogResult<()> {
        let mut executions = self.executions.write().await;
        let tenant_executions = executions.entry(ctx.tenant_id()).or_default();
        let Some(existing) = tenant_executions
            .iter_mut()
            .find(|stored| stored.execution_id() == result.execution_id())
        else {
            return Err(HookExecutionLogError::invalid_persisted_data(format!(
                "missing pending hook execution for {}",
                result.execution_id()
            )));
        };
        *existing = result.clone();
        Ok(())
    }

    async fn store(
        &self,
        ctx: &RequestContext,
        result: &HookExecutionResult,
    ) -> HookExecutionLogResult<()> {
        let mut executions = self.executions.write().await;
        let tenant_executions = executions.entry(ctx.tenant_id()).or_default();
        if tenant_executions.iter().any(|stored| {
            stored.trigger_context_id() == result.trigger_context_id()
                && stored.hook_id() == result.hook_id()
        }) {
            return Err(HookExecutionLogError::persistence_failed(
                std::io::Error::other(format!(
                    "duplicate key value violates unique constraint \"hook_executions_tenant_context_hook_unique\" for trigger context {} and hook {}",
                    result.trigger_context_id(),
                    result.hook_id()
                )),
            ));
        }
        tenant_executions.push(result.clone());
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
