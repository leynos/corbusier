//! Port contract for hook definition lookup.

use crate::hook_engine::domain::{HookDefinition, HookTriggerType};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for hook definition repository operations.
pub type HookDefinitionRepositoryResult<T> = Result<T, HookDefinitionRepositoryError>;

/// Hook definition lookup contract.
#[async_trait]
pub trait HookDefinitionRepository: Send + Sync {
    /// Lists enabled hook definitions for the given trigger type.
    ///
    /// Example: `list_enabled_for_trigger(HookTriggerType::PreCommit)` returns
    /// all enabled pre-commit hooks.
    ///
    /// # Errors
    ///
    /// Returns [`HookDefinitionRepositoryError`] when persistence fails.
    async fn list_enabled_for_trigger(
        &self,
        trigger: HookTriggerType,
    ) -> HookDefinitionRepositoryResult<Vec<HookDefinition>>;
}

/// Errors returned by hook definition repository implementations.
#[derive(Debug, Clone, Error)]
pub enum HookDefinitionRepositoryError {
    /// Persistence-layer failure.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl HookDefinitionRepositoryError {
    /// Wraps a persistence error.
    ///
    /// Example: `HookDefinitionRepositoryError::persistence(err)` wraps `err`.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
