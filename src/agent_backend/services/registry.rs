//! Service layer for agent backend registration and discovery.
//!
//! Provides [`BackendRegistryService`] which coordinates backend registration,
//! deactivation, activation, and discovery operations.

use crate::agent_backend::{
    domain::{
        AgentBackendRegistration, AgentCapabilities, BackendDomainError, BackendId, BackendInfo,
        BackendName,
    },
    ports::{BackendRegistryError, BackendRegistryRepository},
};
use mockable::Clock;
use std::sync::Arc;
use thiserror::Error;

/// Request payload for registering a new agent backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterBackendRequest {
    name: String,
    display_name: String,
    version: String,
    provider: String,
    supports_streaming: bool,
    supports_tool_calls: bool,
    content_types: Vec<String>,
    max_context_window: Option<u64>,
}

impl RegisterBackendRequest {
    /// Creates a request with required backend fields.
    #[expect(
        clippy::too_many_arguments,
        reason = "registration bundles all mandatory fields for a single domain aggregate"
    )]
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        display_name: impl Into<String>,
        version: impl Into<String>,
        provider: impl Into<String>,
        supports_streaming: bool,
        supports_tool_calls: bool,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            version: version.into(),
            provider: provider.into(),
            supports_streaming,
            supports_tool_calls,
            content_types: Vec::new(),
            max_context_window: None,
        }
    }

    /// Sets supported content types.
    #[must_use]
    pub fn with_content_types(mut self, types: impl IntoIterator<Item = String>) -> Self {
        self.content_types = types.into_iter().collect();
        self
    }

    /// Sets the maximum context window size.
    #[must_use]
    pub const fn with_max_context_window(mut self, tokens: u64) -> Self {
        self.max_context_window = Some(tokens);
        self
    }
}

/// Service-level errors for backend registry operations.
#[derive(Debug, Error)]
pub enum BackendRegistryServiceError {
    /// Domain validation failed.
    #[error(transparent)]
    Domain(#[from] BackendDomainError),
    /// Repository operation failed.
    #[error(transparent)]
    Repository(#[from] BackendRegistryError),
}

/// Result type for backend registry service operations.
pub type BackendRegistryServiceResult<T> = Result<T, BackendRegistryServiceError>;

/// Backend registration and discovery orchestration service.
#[derive(Clone)]
pub struct BackendRegistryService<R, C>
where
    R: BackendRegistryRepository,
    C: Clock + Send + Sync,
{
    repository: Arc<R>,
    clock: Arc<C>,
}

impl<R, C> BackendRegistryService<R, C>
where
    R: BackendRegistryRepository,
    C: Clock + Send + Sync,
{
    /// Creates a new backend registry service.
    #[must_use]
    pub const fn new(repository: Arc<R>, clock: Arc<C>) -> Self {
        Self { repository, clock }
    }

    /// Registers a new agent backend.
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryServiceError`] when input validation fails or
    /// the repository rejects persistence.
    pub async fn register(
        &self,
        request: RegisterBackendRequest,
    ) -> BackendRegistryServiceResult<AgentBackendRegistration> {
        let RegisterBackendRequest {
            name,
            display_name,
            version,
            provider,
            supports_streaming,
            supports_tool_calls,
            content_types,
            max_context_window,
        } = request;

        let backend_name = BackendName::new(name)?;
        let backend_info = BackendInfo::new(display_name, version, provider)?;
        let mut capabilities = AgentCapabilities::new(supports_streaming, supports_tool_calls)
            .with_content_types(content_types);
        if let Some(tokens) = max_context_window {
            capabilities = capabilities.with_max_context_window(tokens);
        }

        let registration =
            AgentBackendRegistration::new(backend_name, capabilities, backend_info, &*self.clock);
        self.repository.register(&registration).await?;
        Ok(registration)
    }

    /// Finds a backend registration by internal identifier.
    ///
    /// Returns `Ok(None)` when no backend has the given ID.
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryServiceError::Repository`] when persistence
    /// lookup fails.
    pub async fn find_by_id(
        &self,
        id: BackendId,
    ) -> BackendRegistryServiceResult<Option<AgentBackendRegistration>> {
        Ok(self.repository.find_by_id(id).await?)
    }

    /// Finds a backend registration by unique name.
    ///
    /// Returns `Ok(None)` when no backend has the given name.
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryServiceError::Domain`] when the name string
    /// fails validation, or [`BackendRegistryServiceError::Repository`] when
    /// persistence lookup fails.
    pub async fn find_by_name(
        &self,
        name: &str,
    ) -> BackendRegistryServiceResult<Option<AgentBackendRegistration>> {
        let backend_name = BackendName::new(name)?;
        Ok(self.repository.find_by_name(&backend_name).await?)
    }

    /// Returns all backend registrations with `Active` status.
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryServiceError::Repository`] when persistence
    /// lookup fails.
    pub async fn list_active(&self) -> BackendRegistryServiceResult<Vec<AgentBackendRegistration>> {
        Ok(self.repository.list_active().await?)
    }

    /// Returns all backend registrations regardless of status.
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryServiceError::Repository`] when persistence
    /// lookup fails.
    pub async fn list_all(&self) -> BackendRegistryServiceResult<Vec<AgentBackendRegistration>> {
        Ok(self.repository.list_all().await?)
    }

    /// Deactivates a backend, setting its status to `Inactive`.
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryServiceError::Repository`] when the backend is
    /// not found or persistence fails.
    pub async fn deactivate(
        &self,
        id: BackendId,
    ) -> BackendRegistryServiceResult<AgentBackendRegistration> {
        let mut registration = self.find_by_id_or_error(id).await?;
        registration.deactivate(&*self.clock);
        self.repository.update(&registration).await?;
        Ok(registration)
    }

    /// Activates a backend, setting its status to `Active`.
    ///
    /// # Errors
    ///
    /// Returns [`BackendRegistryServiceError::Repository`] when the backend is
    /// not found or persistence fails.
    pub async fn activate(
        &self,
        id: BackendId,
    ) -> BackendRegistryServiceResult<AgentBackendRegistration> {
        let mut registration = self.find_by_id_or_error(id).await?;
        registration.activate(&*self.clock);
        self.repository.update(&registration).await?;
        Ok(registration)
    }

    async fn find_by_id_or_error(
        &self,
        id: BackendId,
    ) -> BackendRegistryServiceResult<AgentBackendRegistration> {
        self.repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| BackendRegistryError::NotFound(id).into())
    }
}
