//! Port contract for persisting hook execution results.

use crate::hook_engine::domain::{HookExecutionResult, TriggerContextId};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for hook execution log operations.
pub type HookExecutionLogResult<T> = Result<T, HookExecutionLogError>;

/// Hook execution log contract.
#[async_trait]
pub trait HookExecutionLogRepository: Send + Sync {
    /// Persists a hook execution result.
    ///
    /// Example: `store(&result)` records the hook execution.
    ///
    /// # Errors
    ///
    /// Returns [`HookExecutionLogError`] when persistence fails.
    async fn store(&self, result: &HookExecutionResult) -> HookExecutionLogResult<()>;

    /// Returns all execution results for a trigger context.
    ///
    /// Example: `find_by_trigger_context(id)` returns results for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`HookExecutionLogError`] when persistence lookup fails.
    async fn find_by_trigger_context(
        &self,
        trigger_context_id: TriggerContextId,
    ) -> HookExecutionLogResult<Vec<HookExecutionResult>>;
}

/// Errors returned by hook execution log implementations.
#[derive(Debug, Clone, Error)]
pub enum HookExecutionLogError {
    /// Persistence-layer failure.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
    /// Persisted data failed validation.
    #[error("invalid persisted hook execution data: {0}")]
    InvalidPersistedData(String),
}

impl HookExecutionLogError {
    /// Wraps a persistence error.
    ///
    /// Example: `HookExecutionLogError::persistence(err)` wraps `err`.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }

    /// Creates an invalid persisted data error.
    ///
    /// Example: `invalid_persisted_data(\"bad status\")` records bad data.
    pub fn invalid_persisted_data(err: impl Into<String>) -> Self {
        Self::InvalidPersistedData(err.into())
    }
}
