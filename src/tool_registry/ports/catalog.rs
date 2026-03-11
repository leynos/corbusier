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

    /// Returns all catalog entries matching a tool name.
    ///
    /// Multiple entries are returned when more than one server advertises
    /// a tool with the same name; callers must handle the ambiguity.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError`] on persistence failures.
    async fn find_by_tool_name(
        &self,
        ctx: &RequestContext,
        tool_name: &str,
    ) -> ToolCatalogResult<Vec<CatalogEntry>>;

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
    #[error("duplicate catalog entry '{tool_name}' found on {server_count} servers (entry: {id})")]
    DuplicateEntry {
        /// Identifier of the entry that triggered the duplicate detection.
        id: CatalogEntryId,
        /// Conflicting tool name.
        tool_name: String,
        /// Number of servers advertising the conflicting tool name.
        server_count: usize,
    },

    /// A single server reported the same tool name more than once in a batch.
    #[error("duplicate tool '{tool_name}' appeared {entry_count} times in one batch (entry: {id})")]
    DuplicateWithinBatch {
        /// Identifier of the entry that triggered the duplicate detection.
        id: CatalogEntryId,
        /// Conflicting tool name.
        tool_name: String,
        /// Number of times the tool name appeared in the batch.
        entry_count: usize,
    },

    /// No catalog entry matched the query.
    #[error("catalog entry not found: {0}")]
    NotFound(String),

    /// A batch of entries contained invalid or inconsistent data.
    #[error("mixed server batch: {reason}")]
    MixedServerBatch {
        /// Reason for the rejection.
        reason: String,
    },

    /// Persisted data could not be reconstructed into domain types.
    #[error("invalid persisted catalog data for field '{field}': {reason}")]
    InvalidPersistedData {
        /// Name of the field that failed reconstruction.
        field: String,
        /// Reason for the failure.
        reason: String,
    },

    /// A persistence-layer operation failed.
    #[error("catalog persistence error during '{operation}': {reason}")]
    Persistence {
        /// Operation that failed (e.g. "insert", "select").
        operation: String,
        /// Reason for the failure.
        reason: String,
    },
}

impl ToolCatalogError {
    /// Wraps a persistence error from the catalogue adapter.
    pub fn persistence(operation: impl Into<String>, err: impl std::fmt::Display) -> Self {
        Self::Persistence {
            operation: operation.into(),
            reason: err.to_string(),
        }
    }

    /// Wraps an invalid-persisted-data error from the catalogue adapter.
    pub fn invalid_persisted_data(field: impl Into<String>, err: impl std::fmt::Display) -> Self {
        Self::InvalidPersistedData {
            field: field.into(),
            reason: err.to_string(),
        }
    }
}
