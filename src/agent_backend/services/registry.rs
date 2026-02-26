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
    /// Backend identifier string (validated as [`BackendName`] on registration).
    pub name: String,
    /// Human-readable display name for the backend.
    pub display_name: String,
    /// Version string of the backend.
    pub version: String,
    /// Provider/vendor of the backend.
    pub provider: String,
    /// Whether the backend supports streaming responses.
    pub supports_streaming: bool,
    /// Whether the backend supports tool call invocations.
    pub supports_tool_calls: bool,
    /// MIME types the backend can handle.
    pub content_types: Vec<String>,
    /// Maximum context window size in tokens, if known.
    pub max_context_window: Option<u64>,
}

impl RegisterBackendRequest {
    /// Creates a request with required backend fields.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        display_name: impl Into<String>,
        version: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            display_name: display_name.into(),
            version: version.into(),
            provider: provider.into(),
            supports_streaming: false,
            supports_tool_calls: false,
            content_types: Vec::new(),
            max_context_window: None,
        }
    }

    /// Sets the streaming and tool call capabilities.
    #[must_use]
    pub const fn with_capabilities(
        mut self,
        supports_streaming: bool,
        supports_tool_calls: bool,
    ) -> Self {
        self.supports_streaming = supports_streaming;
        self.supports_tool_calls = supports_tool_calls;
        self
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
    /// No backend exists with the given identifier.
    #[error("backend {0} not found")]
    NotFound(BackendId),
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
    /// Returns [`BackendRegistryServiceError::NotFound`] when no backend has
    /// the given ID, or [`BackendRegistryServiceError::Repository`] when
    /// persistence fails.
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
    /// Returns [`BackendRegistryServiceError::NotFound`] when no backend has
    /// the given ID, or [`BackendRegistryServiceError::Repository`] when
    /// persistence fails.
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
            .ok_or(BackendRegistryServiceError::NotFound(id))
    }
}
