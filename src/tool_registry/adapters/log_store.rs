//! Object store adapter for tool stderr log capture and retrieval.

use crate::tool_registry::{
    domain::{LogEntryMetadata, LogRetentionPolicy, McpServerId},
    ports::{ToolLogStore, ToolLogStoreError, ToolLogStoreResult},
};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
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
}

#[async_trait]
impl ToolLogStore for ObjectStoreLogAdapter {
    async fn store_log(
        &self,
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

    async fn retrieve_log(&self, path: &str) -> ToolLogStoreResult<Bytes> {
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

    async fn delete_log(&self, path: &str) -> ToolLogStoreResult<()> {
        let object_path = Path::from(path);
        self.store
            .delete(&object_path)
            .await
            .map_err(|err| ToolLogStoreError::DeleteFailed(err.to_string()))
    }

    async fn list_logs_for_server(
        &self,
        server_id: McpServerId,
    ) -> ToolLogStoreResult<Vec<String>> {
        let prefix = Path::from(format!("tool_logs/{server_id}/"));

        // Use list_with_delimiter to avoid needing futures::TryStreamExt.
        let list_result = self
            .store
            .list_with_delimiter(Some(&prefix))
            .await
            .map_err(|err| ToolLogStoreError::ListFailed(err.to_string()))?;

        let paths = list_result
            .objects
            .iter()
            .map(|meta| meta.location.to_string())
            .collect();

        Ok(paths)
    }

    async fn sweep_expired(
        &self,
        server_id: McpServerId,
        policy: &LogRetentionPolicy,
        now: DateTime<Utc>,
        entry_metadata: &[LogEntryMetadata],
    ) -> ToolLogStoreResult<usize> {
        let mut deleted = 0usize;

        // Delete expired entries.
        for entry in entry_metadata {
            if policy.is_expired(entry, now) {
                self.delete_log(entry.object_path()).await?;
                deleted = deleted.saturating_add(1);
            }
        }

        // Enforce max count per server: delete oldest first.
        let remaining: Vec<&LogEntryMetadata> = entry_metadata
            .iter()
            .filter(|e| e.server_id() == server_id && !policy.is_expired(e, now))
            .collect();

        if remaining.len() > policy.max_logs_per_server {
            let excess = remaining.len().saturating_sub(policy.max_logs_per_server);
            let mut by_time: Vec<&LogEntryMetadata> = remaining;
            by_time.sort_by_key(|e| e.captured_at());
            for entry in by_time.into_iter().take(excess) {
                self.delete_log(entry.object_path()).await?;
                deleted = deleted.saturating_add(1);
            }
        }

        Ok(deleted)
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
    let truncation_point = max.saturating_sub(marker.len());
    let mut truncated = content.slice(..truncation_point).to_vec();
    truncated.extend_from_slice(marker);
    Bytes::from(truncated)
}
