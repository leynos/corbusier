//! Repository port for agent backend registration persistence and discovery.

use crate::agent_backend::domain::{AgentBackendRegistration, BackendId, BackendName};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for backend registry operations.
pub type BackendRegistryResult<T> = Result<T, BackendRegistryError>;

/// Backend registry persistence contract.
#[async_trait]
pub trait BackendRegistryRepository: Send + Sync {
    /// Stores a new backend registration.
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryError::DuplicateBackend`] when the backend ID
    /// already exists or [`BackendRegistryError::DuplicateBackendName`] when
    /// the name is already registered.
    async fn register(&self, registration: &AgentBackendRegistration) -> BackendRegistryResult<()>;

    /// Persists changes to an existing backend registration (status,
    /// capabilities, timestamps).
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryError::NotFound`] when the backend does not
    /// exist.
    async fn update(&self, registration: &AgentBackendRegistration) -> BackendRegistryResult<()>;

    /// Finds a backend registration by internal identifier.
    ///
    /// Returns `None` when the backend does not exist.
    async fn find_by_id(
        &self,
        id: BackendId,
    ) -> BackendRegistryResult<Option<AgentBackendRegistration>>;

    /// Finds a backend registration by unique name.
    ///
    /// Returns `None` when no backend has the given name.
    async fn find_by_name(
        &self,
        name: &BackendName,
    ) -> BackendRegistryResult<Option<AgentBackendRegistration>>;

    /// Returns all backend registrations with `Active` status.
    async fn list_active(&self) -> BackendRegistryResult<Vec<AgentBackendRegistration>>;

    /// Returns all backend registrations regardless of status.
    async fn list_all(&self) -> BackendRegistryResult<Vec<AgentBackendRegistration>>;
}

/// Errors returned by backend registry repository implementations.
#[derive(Debug, Clone, Error)]
pub enum BackendRegistryError {
    /// A backend with the same identifier already exists.
    #[error("duplicate backend identifier: {0}")]
    DuplicateBackend(BackendId),

    /// A backend with the same name already exists.
    #[error("duplicate backend name: {0}")]
    DuplicateBackendName(BackendName),

    /// The backend was not found.
    #[error("backend not found: {0}")]
    NotFound(BackendId),

    /// Persisted data could not be reconstructed into domain types.
    #[error("invalid persisted data: {0}")]
    InvalidPersistedData(Arc<dyn std::error::Error + Send + Sync>),

    /// Persistence-layer failure.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl BackendRegistryError {
    /// Wraps a data-quality or deserialization error from persisted rows.
    pub fn invalid_persisted_data(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::InvalidPersistedData(Arc::new(err))
    }

    /// Wraps a persistence error.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
