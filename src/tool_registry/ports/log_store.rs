//! Port contract for tool stderr log blob storage.
//!
//! The [`ToolLogStore`] trait wraps object storage operations behind
//! the hexagonal boundary, using domain types and `bytes::Bytes`
//! rather than infrastructure-specific types.

use crate::context::RequestContext;
use crate::tool_registry::domain::{LogEntryMetadata, LogRetentionPolicy, McpServerId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Result type for log store operations.
pub type ToolLogStoreResult<T> = Result<T, ToolLogStoreError>;

/// Context bundle for a log retention sweep.
///
/// Groups the policy and wall-clock timestamp that
/// [`ToolLogStore::sweep_expired`] needs, keeping the trait method
/// to two non-`self` parameters.
pub struct SweepContext<'a> {
    /// Retention policy governing expiry and count limits.
    pub policy: &'a LogRetentionPolicy,
    /// Current wall-clock time used for expiry checks.
    pub now: DateTime<Utc>,
}

/// Storage contract for captured stderr log blobs.
///
/// Implementations store and retrieve opaque byte blobs keyed by
/// object store paths derived from [`LogEntryMetadata`].
#[async_trait]
pub trait ToolLogStore: Send + Sync {
    /// Writes a log blob to the store.
    ///
    /// If the content exceeds the retention policy's
    /// `max_bytes_per_log`, the implementation truncates at the byte
    /// boundary. A truncation marker is appended only when the byte
    /// limit is larger than the marker itself.
    ///
    /// # Errors
    ///
    /// Returns [`ToolLogStoreError::StoreFailed`] when the write fails.
    #[expect(
        clippy::too_many_arguments,
        reason = "RequestContext plumbing adds one parameter beyond the natural arity"
    )]
    async fn store_log(
        &self,
        ctx: &RequestContext,
        metadata: &LogEntryMetadata,
        content: bytes::Bytes,
        retention: &LogRetentionPolicy,
    ) -> ToolLogStoreResult<()>;

    /// Reads a log blob by path.
    ///
    /// # Errors
    ///
    /// Returns [`ToolLogStoreError::RetrieveFailed`] when the read
    /// fails.
    async fn retrieve_log(
        &self,
        ctx: &RequestContext,
        path: &str,
    ) -> ToolLogStoreResult<bytes::Bytes>;

    /// Deletes a single log blob by path.
    ///
    /// # Errors
    ///
    /// Returns [`ToolLogStoreError::DeleteFailed`] when the deletion
    /// fails.
    async fn delete_log(&self, ctx: &RequestContext, path: &str) -> ToolLogStoreResult<()>;

    /// Lists all log blob paths for a server by prefix scan.
    ///
    /// # Errors
    ///
    /// Returns [`ToolLogStoreError::ListFailed`] when the listing
    /// fails.
    async fn list_logs_for_server(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolLogStoreResult<Vec<String>>;

    /// Deletes expired logs and enforces the per-server count limit.
    ///
    /// Implementations are responsible for maintaining any metadata
    /// index needed to determine which blobs to delete; callers do not
    /// supply entry metadata.
    ///
    /// Returns the number of entries deleted.
    ///
    /// # Errors
    ///
    /// Returns [`ToolLogStoreError`] when individual deletions fail.
    async fn sweep_expired(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        sweep: &SweepContext<'_>,
    ) -> ToolLogStoreResult<usize>;
}

/// Errors returned by log store operations.
#[derive(Debug, Error)]
pub enum ToolLogStoreError {
    /// Writing a log blob failed.
    #[error("log store write failed: {0}")]
    StoreFailed(String),

    /// Reading a log blob failed.
    #[error("log store read failed: {0}")]
    RetrieveFailed(String),

    /// Deleting a log blob failed.
    #[error("log store delete failed: {0}")]
    DeleteFailed(String),

    /// Listing log entries failed.
    #[error("log store list failed: {0}")]
    ListFailed(String),

    /// The object path does not belong to the expected tenant.
    #[error(
        "tenant mismatch: path '{path}' does not start with expected prefix '{expected_prefix}'"
    )]
    TenantMismatch {
        /// The object path that was rejected.
        path: String,
        /// The expected tenant-scoped prefix.
        expected_prefix: String,
    },
}
