//! Port contract for executing hook actions.

use crate::hook_engine::domain::{ActionResult, HookAction, HookTriggerContext};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for hook action execution.
pub type HookActionExecutionResult<T> = Result<T, HookActionExecutionError>;

/// Hook action execution contract.
#[async_trait]
pub trait HookActionExecutor: Send + Sync {
    /// Executes a single hook action for the given trigger context.
    ///
    /// Example: `executor.execute(action, context)` returns an `ActionResult`.
    ///
    /// # Errors
    ///
    /// Returns [`HookActionExecutionError`] when execution fails.
    async fn execute(
        &self,
        action: &HookAction,
        context: &HookTriggerContext,
    ) -> HookActionExecutionResult<ActionResult>;
}

/// Errors returned by hook action executors.
#[derive(Debug, Clone, Error)]
pub enum HookActionExecutionError {
    /// The action could not be executed.
    #[error("action execution failed: {0}")]
    ExecutionFailed(String),
    /// Execution dependency failure.
    #[error("execution error: {0}")]
    Execution(Arc<dyn std::error::Error + Send + Sync>),
}

impl HookActionExecutionError {
    /// Wraps an execution dependency error.
    ///
    /// Example: `HookActionExecutionError::execution(err)` wraps `err`.
    pub fn execution(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Execution(Arc::new(err))
    }
}
