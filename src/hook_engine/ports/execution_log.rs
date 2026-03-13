//! Port contract for persisting hook execution results.

use crate::context::RequestContext;
use crate::hook_engine::domain::{HookExecutionResult, TriggerContextId};
use async_trait::async_trait;
use thiserror::Error;

/// Result type for hook execution log operations.
pub type HookExecutionLogResult<T> = Result<T, HookExecutionLogError>;

/// Hook execution log contract.
#[async_trait]
pub trait HookExecutionLogRepository: Send + Sync {
    /// Persists a hook execution result.
    ///
    /// Example: `store(&ctx, &result)` records the hook execution.
    ///
    /// # Errors
    ///
    /// Returns [`HookExecutionLogError`] when persistence fails.
    async fn store(
        &self,
        ctx: &RequestContext,
        result: &HookExecutionResult,
    ) -> HookExecutionLogResult<()>;

    /// Returns all execution results for a trigger context.
    ///
    /// Example: `find_by_trigger_context(&ctx, id)` returns results for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`HookExecutionLogError`] when persistence lookup fails.
    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookExecutionLogResult<Vec<HookExecutionResult>>;
}

/// Errors returned by hook execution log implementations.
#[derive(Debug, Clone, Error)]
pub enum HookExecutionLogError {
    /// Persistence-layer failure.
    #[error("persistence operation failed: {reason}")]
    PersistenceFailed {
        /// Human-readable reason from the failing persistence dependency.
        reason: String,
    },
    /// Persisted data failed validation.
    #[error("invalid persisted hook execution data: {0}")]
    InvalidPersistedData(String),
}

impl HookExecutionLogError {
    /// Creates a persistence failure from an infrastructure error.
    ///
    /// Example: `HookExecutionLogError::persistence_failed(err)` stores the
    /// dependency error reason.
    pub fn persistence_failed(err: impl std::error::Error) -> Self {
        Self::PersistenceFailed {
            reason: err.to_string(),
        }
    }

    /// Creates an invalid persisted data error.
    ///
    /// Example: `invalid_persisted_data(\"bad status\")` records bad data.
    pub fn invalid_persisted_data(err: impl Into<String>) -> Self {
        Self::InvalidPersistedData(err.into())
    }
}
