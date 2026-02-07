//! In-memory implementation of the `ContextSnapshotPort`.
//!
//! Provides a simple, thread-safe adapter for unit testing
//! without database dependencies.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use uuid::Uuid;

use crate::message::{
    domain::{AgentSessionId, ContextWindowSnapshot, ConversationId},
    ports::context_snapshot::{ContextSnapshotPort, SnapshotError, SnapshotResult},
};

/// In-memory implementation of [`ContextSnapshotPort`].
///
/// Thread-safe via internal [`RwLock`]. Suitable for unit tests only.
#[derive(Debug, Clone)]
pub struct InMemoryContextSnapshotAdapter {
    snapshots: Arc<RwLock<HashMap<Uuid, ContextWindowSnapshot>>>,
}

impl Default for InMemoryContextSnapshotAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryContextSnapshotAdapter {
    /// Creates a new adapter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns the number of stored snapshots.
    #[must_use]
    pub fn len(&self) -> usize {
        self.snapshots.read().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Returns `true` if no snapshots are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl ContextSnapshotPort for InMemoryContextSnapshotAdapter {
    async fn store_snapshot(&self, snapshot: &ContextWindowSnapshot) -> SnapshotResult<()> {
        let mut guard = self
            .snapshots
            .write()
            .map_err(|e| SnapshotError::persistence(std::io::Error::other(e.to_string())))?;

        if guard.contains_key(&snapshot.snapshot_id) {
            return Err(SnapshotError::Duplicate(snapshot.snapshot_id));
        }

        guard.insert(snapshot.snapshot_id, snapshot.clone());
        Ok(())
    }

    async fn find_by_id(&self, snapshot_id: Uuid) -> SnapshotResult<Option<ContextWindowSnapshot>> {
        let guard = self
            .snapshots
            .read()
            .map_err(|e| SnapshotError::persistence(std::io::Error::other(e.to_string())))?;

        Ok(guard.get(&snapshot_id).cloned())
    }

    async fn find_snapshots_for_session(
        &self,
        session_id: AgentSessionId,
    ) -> SnapshotResult<Vec<ContextWindowSnapshot>> {
        let guard = self
            .snapshots
            .read()
            .map_err(|e| SnapshotError::persistence(std::io::Error::other(e.to_string())))?;

        let mut snapshots: Vec<ContextWindowSnapshot> = guard
            .values()
            .filter(|s| s.session_id == session_id)
            .cloned()
            .collect();

        snapshots.sort_by_key(|s| s.captured_at);
        Ok(snapshots)
    }

    async fn find_latest_snapshot(
        &self,
        conversation_id: ConversationId,
    ) -> SnapshotResult<Option<ContextWindowSnapshot>> {
        let guard = self
            .snapshots
            .read()
            .map_err(|e| SnapshotError::persistence(std::io::Error::other(e.to_string())))?;

        Ok(guard
            .values()
            .filter(|s| s.conversation_id == conversation_id)
            .max_by_key(|s| s.captured_at)
            .cloned())
    }
}
