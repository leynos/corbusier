//! Shared types for tool discovery and routing services.

use crate::tool_registry::{
    domain::{McpServerId, ToolRegistryDomainError},
    ports::{
        McpServerHostError, McpServerRegistryError, ToolCatalogError, ToolGovernanceError,
        ToolLogStoreError,
    },
};
use std::sync::Arc;
use thiserror::Error;

/// Service-level errors for tool discovery and routing operations.
#[derive(Debug, Error)]
pub enum ToolDiscoveryRoutingServiceError {
    /// Domain validation failed.
    #[error(transparent)]
    Domain(#[from] ToolRegistryDomainError),
    /// Catalog persistence failed.
    #[error(transparent)]
    Catalog(#[from] ToolCatalogError),
    /// Registry operation failed.
    #[error(transparent)]
    Registry(#[from] McpServerRegistryError),
    /// Host operation failed.
    #[error(transparent)]
    Host(#[from] McpServerHostError),
    /// Tool governance returned an error.
    #[error(transparent)]
    Governance(#[from] ToolGovernanceError),
    /// Log store operation failed.
    #[error(transparent)]
    LogStore(#[from] ToolLogStoreError),
    /// No server exists with the given identifier.
    #[error("MCP server {0} not found")]
    NotFound(McpServerId),
}

/// Result type for discovery and routing service operations.
pub type ToolDiscoveryRoutingServiceResult<T> = Result<T, ToolDiscoveryRoutingServiceError>;

/// Port dependencies for the tool discovery routing service.
#[derive(Debug)]
pub struct ServicePorts<Cat, Reg, H, Gov, Log> {
    /// Catalog repository.
    pub catalog: Arc<Cat>,
    /// Server registry.
    pub registry: Arc<Reg>,
    /// Server host.
    pub host: Arc<H>,
    /// Tool execution governance.
    pub governance: Arc<Gov>,
    /// Log store.
    pub log_store: Arc<Log>,
}
