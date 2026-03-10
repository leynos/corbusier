//! Object store adapter for tool stderr log capture and retrieval.
//!
//! The adapter maintains an in-memory metadata index alongside the
//! blob store so that [`ToolLogStore::sweep_expired`] can identify
//! expired or excess entries without relying on the caller to supply
//! the full metadata slice (which the service layer cannot provide
//! for the object-store backend).

use crate::context::RequestContext;
use crate::tool_registry::{
    domain::{LogEntryMetadata, LogRetentionPolicy, McpServerId},
    ports::{SweepContext, ToolLogStore, ToolLogStoreError, ToolLogStoreResult},
};
use async_trait::async_trait;
use bytes::Bytes;
use object_store::{ObjectStore, path::Path};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Adapter wrapping an [`ObjectStore`] backend for tool stderr log storage.
///
/// Supports any `object_store` backend: `InMemory` for tests,
/// `LocalFileSystem` for development, and cloud backends (S3, GCS)
/// for production use.
///
/// An in-memory metadata index tracks every stored entry so that
/// retention sweeps can operate without external metadata.
#[derive(Debug, Clone)]
pub struct ObjectStoreLogAdapter {
    store: Arc<dyn ObjectStore>,
    /// In-memory metadata index keyed by `object_path`.
    metadata: Arc<RwLock<HashMap<String, LogEntryMetadata>>>,
}

impl ObjectStoreLogAdapter {
    /// Creates a new log adapter from any [`ObjectStore`] implementation.
    #[must_use]
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self {
            store,
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates an in-memory backed adapter for tests.
    #[must_use]
    pub fn in_memory() -> Self {
        Self {
            store: Arc::new(object_store::memory::InMemory::new()),
            metadata: Arc::new(RwLock::new(HashMap::new())),
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

    /// Collects metadata entries for `server_id` by merging the
    /// internal in-memory index with `sweep.entry_metadata`.
    ///
    /// In-memory entries take precedence when both sources contain
    /// the same `object_path`.
    async fn collect_server_metadata(
        &self,
        server_id: McpServerId,
        sweep: &SweepContext<'_>,
    ) -> Vec<LogEntryMetadata> {
        let guard = self.metadata.read().await;
        let mut merged: HashMap<String, LogEntryMetadata> = sweep
            .entry_metadata
            .iter()
            .filter(|e| e.server_id() == server_id)
            .map(|e| (e.object_path().to_owned(), e.clone()))
            .collect();
        // In-memory entries override externally supplied duplicates.
        for entry in guard.values().filter(|e| e.server_id() == server_id) {
            merged.insert(entry.object_path().to_owned(), entry.clone());
        }
        merged.into_values().collect()
    }

    /// Deletes log entries whose retention period has elapsed.
    async fn delete_expired_entries(
        &self,
        entries: &[LogEntryMetadata],
        sweep: &SweepContext<'_>,
    ) -> ToolLogStoreResult<Vec<String>> {
        let mut swept_keys = Vec::new();
        for entry in entries {
            if sweep.policy.is_expired(entry, sweep.now) {
                self.delete_blob(entry.object_path()).await?;
                swept_keys.push(entry.object_path().to_owned());
            }
        }
        Ok(swept_keys)
    }

    /// Removes the oldest logs when a server exceeds its count limit.
    async fn enforce_count_limit(
        &self,
        entries: &[LogEntryMetadata],
        swept_keys: &[String],
        sweep: &SweepContext<'_>,
    ) -> ToolLogStoreResult<Vec<String>> {
        let swept_set: HashSet<&str> = swept_keys.iter().map(String::as_str).collect();
        let mut remaining: Vec<&LogEntryMetadata> = entries
            .iter()
            .filter(|e| {
                !sweep.policy.is_expired(e, sweep.now) && !swept_set.contains(e.object_path())
            })
            .collect();

        let max = sweep.policy.max_logs_per_server;
        if remaining.len() <= max {
            return Ok(Vec::new());
        }

        let excess = remaining.len().saturating_sub(max);
        remaining.sort_by_key(|e| e.captured_at());

        let mut excess_keys = Vec::new();
        for entry in remaining.into_iter().take(excess) {
            self.delete_blob(entry.object_path()).await?;
            excess_keys.push(entry.object_path().to_owned());
        }
        Ok(excess_keys)
    }
}

/// Validates that `path` belongs to the expected tenant.
fn validate_tenant_prefix(ctx: &RequestContext, path: &str) -> ToolLogStoreResult<()> {
    let expected_prefix = format!("tool_logs/{}/", ctx.tenant_id());
    if !path.starts_with(&expected_prefix) {
        return Err(ToolLogStoreError::TenantMismatch {
            path: path.to_owned(),
            expected_prefix,
        });
    }
    Ok(())
}

#[async_trait]
impl ToolLogStore for ObjectStoreLogAdapter {
    async fn store_log(
        &self,
        ctx: &RequestContext,
        metadata: &LogEntryMetadata,
        content: Bytes,
        retention: &LogRetentionPolicy,
    ) -> ToolLogStoreResult<()> {
        validate_tenant_prefix(ctx, metadata.object_path())?;
        let path = Path::from(metadata.object_path());
        let truncated = truncate_if_needed(content, retention.max_bytes_per_log);
        self.store
            .put(&path, truncated.into())
            .await
            .map_err(|err| ToolLogStoreError::StoreFailed(err.to_string()))?;

        // Track metadata for retention sweeps.
        self.metadata
            .write()
            .await
            .insert(metadata.object_path().to_owned(), metadata.clone());
        Ok(())
    }

    async fn retrieve_log(&self, ctx: &RequestContext, path: &str) -> ToolLogStoreResult<Bytes> {
        validate_tenant_prefix(ctx, path)?;
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

    async fn delete_log(&self, ctx: &RequestContext, path: &str) -> ToolLogStoreResult<()> {
        validate_tenant_prefix(ctx, path)?;
        let object_path = Path::from(path);
        self.store
            .delete(&object_path)
            .await
            .map_err(|err| ToolLogStoreError::DeleteFailed(err.to_string()))?;

        // Remove from internal metadata index.
        self.metadata.write().await.remove(path);
        Ok(())
    }

    async fn list_logs_for_server(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolLogStoreResult<Vec<String>> {
        use futures::TryStreamExt;

        let tenant_id = ctx.tenant_id();
        let prefix = Path::from(format!("tool_logs/{tenant_id}/{server_id}/"));

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
        let entries = self.collect_server_metadata(server_id, sweep).await;

        let swept = self.delete_expired_entries(&entries, sweep).await?;
        let excess = self.enforce_count_limit(&entries, &swept, sweep).await?;

        // Purge swept keys from the internal metadata index.
        let total_keys: Vec<String> = swept.into_iter().chain(excess).collect();
        if !total_keys.is_empty() {
            let mut guard = self.metadata.write().await;
            for key in &total_keys {
                guard.remove(key);
            }
        }

        Ok(total_keys.len())
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

#[cfg(test)]
mod tests;
