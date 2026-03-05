//! In-memory repository for agent backend registration tests.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::agent_backend::{
    domain::{AgentBackendRegistration, BackendId, BackendName, BackendStatus},
    ports::{BackendRegistryError, BackendRegistryRepository, BackendRegistryResult},
};
use crate::context::RequestContext;

/// Thread-safe in-memory backend registry repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryBackendRegistry {
    state: Arc<RwLock<InMemoryRegistryState>>,
}

#[derive(Debug, Default)]
struct InMemoryRegistryState {
    backends: HashMap<BackendId, AgentBackendRegistration>,
    name_index: HashMap<BackendName, BackendId>,
}

impl InMemoryBackendRegistry {
    /// Creates an empty in-memory registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn read_state(&self) -> BackendRegistryResult<RwLockReadGuard<'_, InMemoryRegistryState>> {
        self.state.read().map_err(|err| {
            BackendRegistryError::persistence(std::io::Error::other(err.to_string()))
        })
    }

    fn write_state(&self) -> BackendRegistryResult<RwLockWriteGuard<'_, InMemoryRegistryState>> {
        self.state.write().map_err(|err| {
            BackendRegistryError::persistence(std::io::Error::other(err.to_string()))
        })
    }
}

#[async_trait]
impl BackendRegistryRepository for InMemoryBackendRegistry {
    async fn register(
        &self,
        _ctx: &RequestContext,
        registration: &AgentBackendRegistration,
    ) -> BackendRegistryResult<()> {
        let mut state = self.write_state()?;

        if state.backends.contains_key(&registration.id()) {
            return Err(BackendRegistryError::DuplicateBackend(registration.id()));
        }

        if state.name_index.contains_key(registration.name()) {
            return Err(BackendRegistryError::DuplicateBackendName(
                registration.name().clone(),
            ));
        }

        state
            .name_index
            .insert(registration.name().clone(), registration.id());
        state
            .backends
            .insert(registration.id(), registration.clone());
        Ok(())
    }

    async fn update(
        &self,
        _ctx: &RequestContext,
        registration: &AgentBackendRegistration,
    ) -> BackendRegistryResult<()> {
        let mut state = self.write_state()?;

        let old_name = state
            .backends
            .get(&registration.id())
            .ok_or(BackendRegistryError::NotFound(registration.id()))?
            .name()
            .clone();

        if *registration.name() != old_name {
            if let Some(&indexed_id) = state.name_index.get(registration.name())
                && indexed_id != registration.id()
            {
                return Err(BackendRegistryError::DuplicateBackendName(
                    registration.name().clone(),
                ));
            }
            state.name_index.remove(&old_name);
            state
                .name_index
                .insert(registration.name().clone(), registration.id());
        }

        state
            .backends
            .insert(registration.id(), registration.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        _ctx: &RequestContext,
        id: BackendId,
    ) -> BackendRegistryResult<Option<AgentBackendRegistration>> {
        let state = self.read_state()?;
        Ok(state.backends.get(&id).cloned())
    }

    async fn find_by_name(
        &self,
        _ctx: &RequestContext,
        name: &BackendName,
    ) -> BackendRegistryResult<Option<AgentBackendRegistration>> {
        let state = self.read_state()?;
        let backend = state
            .name_index
            .get(name)
            .and_then(|id| state.backends.get(id))
            .cloned();
        Ok(backend)
    }

    async fn list_active(
        &self,
        _ctx: &RequestContext,
    ) -> BackendRegistryResult<Vec<AgentBackendRegistration>> {
        let state = self.read_state()?;
        let active = state
            .backends
            .values()
            .filter(|b| b.status() == BackendStatus::Active)
            .cloned()
            .collect();
        Ok(active)
    }

    async fn list_all(
        &self,
        _ctx: &RequestContext,
    ) -> BackendRegistryResult<Vec<AgentBackendRegistration>> {
        let state = self.read_state()?;
        Ok(state.backends.values().cloned().collect())
    }
}
