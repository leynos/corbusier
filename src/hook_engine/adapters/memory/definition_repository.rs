//! In-memory hook definition repository.

use crate::hook_engine::domain::{HookDefinition, HookTriggerType};
use crate::hook_engine::ports::{HookDefinitionRepository, HookDefinitionRepositoryResult};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    /// Example: `repo.insert(definition).await` stores the definition in memory.
    ///
    /// # Errors
    ///
    /// Returns `HookDefinitionRepositoryError` when persisting the definition
    /// fails.
    pub async fn insert(&self, definition: HookDefinition) -> HookDefinitionRepositoryResult<()> {
        let mut definitions = self.definitions.write().await;
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
        let definitions = self.definitions.read().await;
        Ok(definitions
            .iter()
            .filter(|definition| definition.is_enabled() && definition.trigger() == trigger)
            .cloned()
            .collect())
    }
}
