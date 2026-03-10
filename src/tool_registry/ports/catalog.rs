//! Port contract for tool catalog persistence and audit trail.

use crate::context::RequestContext;
use crate::tool_registry::domain::{
    CatalogEntry, CatalogEntryId, McpServerId, ToolCallAuditRecord,
};
use async_trait::async_trait;
use thiserror::Error;

/// Result type for tool catalog operations.
pub type ToolCatalogResult<T> = Result<T, ToolCatalogError>;

/// Persistence contract for the tool catalog and audit trail.
///
/// Implementations manage the durable storage of discovered tool entries
/// and tool call audit records.
#[async_trait]
pub trait ToolCatalogRepository: Send + Sync {
    /// Persists or updates a batch of catalog entries for a server.
    ///
    /// Existing entries for the server are replaced with the provided set.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError`] on persistence or duplicate entry
    /// failures.
    async fn sync_server_tools(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<()>;

    /// Marks all tools for the given server as unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError`] on persistence failures.
    async fn mark_server_tools_unavailable(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()>;

    /// Marks all tools for the given server as available.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError`] on persistence failures.
    async fn mark_server_tools_available(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()>;

    /// Finds a catalog entry by tool name.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError`] on persistence failures.
    async fn find_by_tool_name(
        &self,
        ctx: &RequestContext,
        tool_name: &str,
    ) -> ToolCatalogResult<Option<CatalogEntry>>;

    /// Returns the complete tool catalog.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError`] on persistence failures.
    async fn list_all(&self, ctx: &RequestContext) -> ToolCatalogResult<Vec<CatalogEntry>>;

    /// Persists a tool call audit trail record.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError`] on persistence failures.
    async fn record_audit(
        &self,
        ctx: &RequestContext,
        record: &ToolCallAuditRecord,
    ) -> ToolCatalogResult<()>;
}

/// Errors returned by tool catalog persistence operations.
#[derive(Debug, Error)]
pub enum ToolCatalogError {
    /// A catalog entry with the same identity already exists.
    #[error("duplicate catalog entry: {0}")]
    DuplicateEntry(CatalogEntryId),

    /// No catalog entry matched the query.
    #[error("catalog entry not found: {0}")]
    NotFound(String),

    /// Persisted data could not be reconstructed into domain types.
    #[error("invalid persisted catalog data: {0}")]
    InvalidPersistedData(String),

    /// A persistence-layer operation failed.
    #[error("catalog persistence error: {0}")]
    Persistence(String),
}

impl ToolCatalogError {
    /// Wraps a persistence error from the catalogue adapter.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(err.to_string())
    }

    /// Wraps an invalid-persisted-data error from the catalogue adapter.
    pub fn invalid_persisted_data(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::InvalidPersistedData(err.to_string())
    }
}
