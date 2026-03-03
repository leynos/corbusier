//! Hook engine execution port.

use super::{HookActionExecutionError, HookDefinitionRepositoryError, HookExecutionLogError};
use crate::hook_engine::domain::{HookExecutionResult, HookTriggerContext, HookTriggerType};
use async_trait::async_trait;
use thiserror::Error;

/// Result type for hook engine execution.
pub type HookEngineResult<T> = Result<T, HookEngineError>;

/// Hook engine execution contract.
#[async_trait]
pub trait HookEngine: Send + Sync {
    /// Executes all configured hooks for the given trigger context.
    ///
    /// Example: `engine.execute(context)` returns execution results.
    ///
    /// # Errors
    ///
    /// Returns [`HookEngineError`] when definition lookup, action execution,
    /// or persistence fails.
    async fn execute(
        &self,
        context: HookTriggerContext,
    ) -> HookEngineResult<Vec<HookExecutionResult>>;

    /// Returns all configured triggers supported by this engine.
    ///
    /// Example: `engine.supported_triggers()` returns the trigger list.
    fn supported_triggers(&self) -> &'static [HookTriggerType];
}

/// Errors returned while executing hooks.
#[derive(Debug, Error)]
pub enum HookEngineError {
    /// Definition repository failure.
    #[error(transparent)]
    DefinitionRepository(#[from] HookDefinitionRepositoryError),
    /// Action execution failure.
    #[error(transparent)]
    ActionExecution(#[from] HookActionExecutionError),
    /// Execution log persistence failure.
    #[error(transparent)]
    ExecutionLog(#[from] HookExecutionLogError),
}
