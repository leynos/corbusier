//! Port contract for executing hook actions.

use crate::hook_engine::domain::{ActionResult, HookAction, HookTriggerContext};
use async_trait::async_trait;
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
    /// Dependency failure while executing an action.
    #[error("action executor dependency failure: {reason}")]
    DependencyFailure {
        /// Human-readable reason from the failing dependency.
        reason: String,
    },
}

impl HookActionExecutionError {
    /// Creates a dependency failure from an infrastructure error.
    ///
    /// Example: `HookActionExecutionError::dependency_failure(err)` records the
    /// dependency error reason.
    pub fn dependency_failure(err: impl std::error::Error) -> Self {
        Self::DependencyFailure {
            reason: err.to_string(),
        }
    }
}
