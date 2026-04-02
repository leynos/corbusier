//! In-memory hook action executor for tests and local runs.

use crate::hook_engine::domain::{
    ActionResult, ActionResultDetails, ActionStatus, HookAction, HookLogEntry, HookLogLevel,
    HookTriggerContext,
};
use crate::hook_engine::ports::{
    HookActionExecutionError, HookActionExecutionResult, HookActionExecutor,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory hook action executor with configurable outcomes.
#[derive(Debug, Clone, Default)]
pub struct InMemoryHookActionExecutor {
    outcomes: Arc<RwLock<HashMap<String, ActionStatus>>>,
    outputs: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl InMemoryHookActionExecutor {
    /// Creates a new in-memory executor with no predefined outcomes.
    ///
    /// Example: `InMemoryHookActionExecutor::new()` creates a default executor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the outcome for a specific action identifier.
    ///
    /// Example: `executor.set_outcome("action-1", ActionStatus::Failed)`
    /// configures a failure for that action.
    ///
    /// # Errors
    ///
    /// Returns [`HookActionExecutionError`] if the outcome lock is unavailable.
    pub fn set_outcome(
        &self,
        action_id: impl Into<String>,
        status: ActionStatus,
    ) -> HookActionExecutionResult<()> {
        let mut outcomes = self.outcomes.try_write().map_err(|err| {
            HookActionExecutionError::dependency_failure(std::io::Error::other(format!(
                "failed to acquire action outcome write lock: {err}"
            )))
        })?;
        outcomes.insert(action_id.into(), status);
        Ok(())
    }

    /// Sets the output payload for a specific action identifier.
    ///
    /// # Errors
    ///
    /// Returns [`HookActionExecutionError`] if the output lock is unavailable.
    pub fn set_output(
        &self,
        action_id: impl Into<String>,
        output: serde_json::Value,
    ) -> HookActionExecutionResult<()> {
        let mut outputs = self.outputs.try_write().map_err(|err| {
            HookActionExecutionError::dependency_failure(std::io::Error::other(format!(
                "failed to acquire action output write lock: {err}"
            )))
        })?;
        outputs.insert(action_id.into(), output);
        Ok(())
    }

    async fn resolve_status(&self, action_id: &str) -> ActionStatus {
        let outcomes = self.outcomes.read().await;
        outcomes
            .get(action_id)
            .copied()
            .unwrap_or(ActionStatus::Succeeded)
    }

    async fn resolve_output(
        &self,
        action_id: &str,
        status: ActionStatus,
        context: &HookTriggerContext,
    ) -> serde_json::Value {
        let outputs = self.outputs.read().await;
        let Some(output) = outputs.get(action_id).cloned() else {
            return serde_json::json!({
                "status": status.as_str(),
                "trigger": context.trigger_type().as_str(),
            });
        };
        output
    }
}

#[async_trait]
impl HookActionExecutor for InMemoryHookActionExecutor {
    async fn execute(
        &self,
        action: &HookAction,
        context: &HookTriggerContext,
    ) -> HookActionExecutionResult<ActionResult> {
        let status = self.resolve_status(action.id().as_str()).await;
        let log_entry = HookLogEntry::new(
            HookLogLevel::Info,
            format!("action {} executed with status {}", action.id(), status),
            context.occurred_at(),
        );
        let output = self
            .resolve_output(action.id().as_str(), status, context)
            .await;
        Ok(ActionResult::new(ActionResultDetails {
            action_id: action.id().clone(),
            action_type: action.action_type().clone(),
            status,
            output,
            log_entries: vec![log_entry],
        }))
    }
}
