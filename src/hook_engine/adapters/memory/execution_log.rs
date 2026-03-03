//! In-memory hook execution log repository.

use crate::hook_engine::domain::{HookExecutionResult, TriggerContextId};
use crate::hook_engine::ports::{
    HookExecutionLogError, HookExecutionLogRepository, HookExecutionLogResult,
};
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

/// Thread-safe in-memory hook execution log repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryHookExecutionLogRepository {
    executions: Arc<RwLock<Vec<HookExecutionResult>>>,
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
    /// Example: `repo.all()` returns the stored executions.
    ///
    /// # Errors
    ///
    /// Returns [`HookExecutionLogError`] if the lock is poisoned.
    pub fn all(&self) -> HookExecutionLogResult<Vec<HookExecutionResult>> {
        let executions = self.executions.read().map_err(|err| {
            HookExecutionLogError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(executions.clone())
    }
}

#[async_trait]
impl HookExecutionLogRepository for InMemoryHookExecutionLogRepository {
    async fn store(&self, result: &HookExecutionResult) -> HookExecutionLogResult<()> {
        let mut executions = self.executions.write().map_err(|err| {
            HookExecutionLogError::persistence(std::io::Error::other(err.to_string()))
        })?;
        executions.push(result.clone());
        Ok(())
    }

    async fn find_by_trigger_context(
        &self,
        trigger_context_id: TriggerContextId,
    ) -> HookExecutionLogResult<Vec<HookExecutionResult>> {
        let executions = self.executions.read().map_err(|err| {
            HookExecutionLogError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(executions
            .iter()
            .filter(|result| result.trigger_context_id() == trigger_context_id)
            .cloned()
            .collect())
    }
}
