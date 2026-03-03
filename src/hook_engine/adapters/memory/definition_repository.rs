//! In-memory hook definition repository.

use crate::hook_engine::domain::{HookDefinition, HookTriggerType};
use crate::hook_engine::ports::{
    HookDefinitionRepository, HookDefinitionRepositoryError, HookDefinitionRepositoryResult,
};
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

/// Thread-safe in-memory hook definition repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryHookDefinitionRepository {
    definitions: Arc<RwLock<Vec<HookDefinition>>>,
}

impl InMemoryHookDefinitionRepository {
    /// Creates an empty in-memory repository.
    ///
    /// Example: `InMemoryHookDefinitionRepository::new()` creates a repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a hook definition into the repository.
    ///
    /// Example: `repo.insert(definition)` stores the definition in memory.
    ///
    /// # Errors
    ///
    /// Returns [`HookDefinitionRepositoryError`] if the lock is poisoned.
    pub fn insert(&self, definition: HookDefinition) -> HookDefinitionRepositoryResult<()> {
        let mut definitions = self.definitions.write().map_err(|err| {
            HookDefinitionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        definitions.push(definition);
        Ok(())
    }
}

#[async_trait]
impl HookDefinitionRepository for InMemoryHookDefinitionRepository {
    async fn list_enabled_for_trigger(
        &self,
        trigger: HookTriggerType,
    ) -> HookDefinitionRepositoryResult<Vec<HookDefinition>> {
        let definitions = self.definitions.read().map_err(|err| {
            HookDefinitionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(definitions
            .iter()
            .filter(|definition| definition.is_enabled() && definition.trigger() == trigger)
            .cloned()
            .collect())
    }
}
