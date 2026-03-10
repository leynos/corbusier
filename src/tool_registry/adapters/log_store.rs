//! Object store adapter for tool stderr log capture and retrieval.

use crate::context::RequestContext;
use crate::tool_registry::{
    domain::{LogEntryMetadata, LogRetentionPolicy, McpServerId},
    ports::{SweepContext, ToolLogStore, ToolLogStoreError, ToolLogStoreResult},
};
use async_trait::async_trait;
use bytes::Bytes;
use object_store::{ObjectStore, path::Path};
use std::sync::Arc;

/// Adapter wrapping an [`ObjectStore`] backend for tool stderr log storage.
///
/// Supports any `object_store` backend: `InMemory` for tests,
/// `LocalFileSystem` for development, and cloud backends (S3, GCS)
/// for production use.
#[derive(Debug, Clone)]
pub struct ObjectStoreLogAdapter {
    store: Arc<dyn ObjectStore>,
}

impl ObjectStoreLogAdapter {
    /// Creates a new log adapter from any [`ObjectStore`] implementation.
    #[must_use]
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self { store }
    }

    /// Creates an in-memory backed adapter for tests.
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            store: Arc::new(object_store::memory::InMemory::new()),
        }
    }

    /// Deletes a blob directly from the object store, bypassing the
    /// trait method (which requires `&RequestContext`).
    async fn delete_blob(&self, path: &str) -> ToolLogStoreResult<()> {
        let object_path = Path::from(path);
        self.store
            .delete(&object_path)
            .await
            .map_err(|err| ToolLogStoreError::DeleteFailed(err.to_string()))
    }

    /// Deletes log entries whose retention period has elapsed.
    async fn delete_expired_entries(&self, sweep: &SweepContext<'_>) -> ToolLogStoreResult<usize> {
        let mut deleted = 0usize;
        for entry in sweep.entry_metadata {
            if sweep.policy.is_expired(entry, sweep.now) {
                self.delete_blob(entry.object_path()).await?;
                deleted = deleted.saturating_add(1);
            }
        }
        Ok(deleted)
    }

    /// Removes the oldest logs when a server exceeds its count limit.
    async fn enforce_count_limit(
        &self,
        server_id: McpServerId,
        sweep: &SweepContext<'_>,
    ) -> ToolLogStoreResult<usize> {
        let mut remaining: Vec<&LogEntryMetadata> = sweep
            .entry_metadata
            .iter()
            .filter(|e| e.server_id() == server_id && !sweep.policy.is_expired(e, sweep.now))
            .collect();

        let max = sweep.policy.max_logs_per_server;
        if remaining.len() <= max {
            return Ok(0);
        }

        let excess = remaining.len().saturating_sub(max);
        remaining.sort_by_key(|e| e.captured_at());

        let mut deleted = 0usize;
        for entry in remaining.into_iter().take(excess) {
            self.delete_blob(entry.object_path()).await?;
            deleted = deleted.saturating_add(1);
        }
        Ok(deleted)
    }
}

#[async_trait]
impl ToolLogStore for ObjectStoreLogAdapter {
    async fn store_log(
        &self,
        _ctx: &RequestContext,
        metadata: &LogEntryMetadata,
        content: Bytes,
        retention: &LogRetentionPolicy,
    ) -> ToolLogStoreResult<()> {
        let path = Path::from(metadata.object_path());
        let truncated = truncate_if_needed(content, retention.max_bytes_per_log);
        self.store
            .put(&path, truncated.into())
            .await
            .map_err(|err| ToolLogStoreError::StoreFailed(err.to_string()))?;
        Ok(())
    }

    async fn retrieve_log(&self, _ctx: &RequestContext, path: &str) -> ToolLogStoreResult<Bytes> {
        let object_path = Path::from(path);
        let result = self
            .store
            .get(&object_path)
            .await
            .map_err(|err| ToolLogStoreError::RetrieveFailed(err.to_string()))?;
        result
            .bytes()
            .await
            .map_err(|err| ToolLogStoreError::RetrieveFailed(err.to_string()))
    }

    async fn delete_log(&self, _ctx: &RequestContext, path: &str) -> ToolLogStoreResult<()> {
        let object_path = Path::from(path);
        self.store
            .delete(&object_path)
            .await
            .map_err(|err| ToolLogStoreError::DeleteFailed(err.to_string()))
    }

    async fn list_logs_for_server(
        &self,
        _ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolLogStoreResult<Vec<String>> {
        use futures::TryStreamExt;

        let prefix = Path::from(format!("tool_logs/{server_id}/"));

        self.store
            .list(Some(&prefix))
            .map_ok(|meta| meta.location.to_string())
            .try_collect()
            .await
            .map_err(|err| ToolLogStoreError::ListFailed(err.to_string()))
    }

    async fn sweep_expired(
        &self,
        _ctx: &RequestContext,
        server_id: McpServerId,
        sweep: &SweepContext<'_>,
    ) -> ToolLogStoreResult<usize> {
        let expired = self.delete_expired_entries(sweep).await?;
        let excess = self.enforce_count_limit(server_id, sweep).await?;
        Ok(expired.saturating_add(excess))
    }
}

/// Truncates content if it exceeds `max_bytes`, appending a truncation marker.
#[expect(
    clippy::cast_possible_truncation,
    reason = "log sizes are well within usize range on all supported platforms"
)]
fn truncate_if_needed(content: Bytes, max_bytes: u64) -> Bytes {
    let max = max_bytes as usize;
    if content.len() <= max {
        return content;
    }
    let marker = b"\n--- truncated at max_bytes_per_log ---\n";
    if max <= marker.len() {
        // Cap is too small for the marker; just hard-truncate.
        return content.slice(..max);
    }
    let truncation_point = max.saturating_sub(marker.len());
    let mut truncated = content.slice(..truncation_point).to_vec();
    truncated.extend_from_slice(marker);
    Bytes::from(truncated)
}
