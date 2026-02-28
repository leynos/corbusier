//! Repository port for MCP server registry persistence and discovery.

use crate::tool_registry::domain::{McpServerId, McpServerName, McpServerRegistration};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for MCP server registry operations.
pub type McpServerRegistryResult<T> = Result<T, McpServerRegistryError>;

/// Persistence contract for MCP server registrations.
#[async_trait]
pub trait McpServerRegistryRepository: Send + Sync {
    /// Stores a new server registration.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerRegistryError::DuplicateServer`] when the ID already
    /// exists or [`McpServerRegistryError::DuplicateServerName`] when the name
    /// is already registered.
    async fn register(&self, server: &McpServerRegistration) -> McpServerRegistryResult<()>;

    /// Persists updates to an existing registration.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerRegistryError::NotFound`] when the server does not
    /// exist.
    async fn update(&self, server: &McpServerRegistration) -> McpServerRegistryResult<()>;

    /// Finds a registration by internal identifier.
    async fn find_by_id(
        &self,
        server_id: McpServerId,
    ) -> McpServerRegistryResult<Option<McpServerRegistration>>;

    /// Finds a registration by unique server name.
    async fn find_by_name(
        &self,
        server_name: &McpServerName,
    ) -> McpServerRegistryResult<Option<McpServerRegistration>>;

    /// Returns all registrations regardless of lifecycle state.
    async fn list_all(&self) -> McpServerRegistryResult<Vec<McpServerRegistration>>;
}

/// Errors returned by MCP server registry repository implementations.
#[derive(Debug, Clone, Error)]
pub enum McpServerRegistryError {
    /// A server with the same identifier already exists.
    #[error("duplicate MCP server identifier: {0}")]
    DuplicateServer(McpServerId),

    /// A server with the same name already exists.
    #[error("duplicate MCP server name: {0}")]
    DuplicateServerName(McpServerName),

    /// The server was not found.
    #[error("MCP server not found: {0}")]
    NotFound(McpServerId),

    /// Persisted data could not be reconstructed into domain types.
    #[error("invalid persisted MCP server data: {0}")]
    InvalidPersistedData(Arc<dyn std::error::Error + Send + Sync>),

    /// Persistence-layer failure.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl McpServerRegistryError {
    /// Wraps persisted-data decoding or validation failures.
    pub fn invalid_persisted_data(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::InvalidPersistedData(Arc::new(err))
    }

    /// Wraps a persistence-layer failure.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
