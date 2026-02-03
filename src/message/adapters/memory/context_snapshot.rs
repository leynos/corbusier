//! In-memory implementation of the `ContextSnapshotPort`.
//!
//! Provides a simple, thread-safe adapter for unit testing
//! without database dependencies.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use mockable::Clock;
use uuid::Uuid;

use crate::message::{
    domain::{
        AgentSessionId, ContextWindowSnapshot, ConversationId, MessageSummary, SequenceNumber,
        SequenceRange, SnapshotParams,
    },
    ports::context_snapshot::{
        CaptureSnapshotParams, ContextSnapshotPort, SnapshotError, SnapshotResult,
    },
};

/// In-memory implementation of [`ContextSnapshotPort`].
///
/// Thread-safe via internal [`RwLock`]. Suitable for unit tests only.
#[derive(Debug, Clone)]
pub struct InMemoryContextSnapshotAdapter<C: Clock + Send + Sync> {
    snapshots: Arc<RwLock<HashMap<Uuid, ContextWindowSnapshot>>>,
    clock: C,
}

impl<C: Clock + Send + Sync> InMemoryContextSnapshotAdapter<C> {
    /// Creates a new adapter with the given clock.
    #[must_use]
    pub fn new(clock: C) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
            clock,
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
impl<C: Clock + Send + Sync> ContextSnapshotPort for InMemoryContextSnapshotAdapter<C> {
    async fn capture_snapshot(
        &self,
        params: CaptureSnapshotParams,
    ) -> SnapshotResult<ContextWindowSnapshot> {
        // In a real implementation, we'd query the message repository
        // to compute the actual summary. For testing, we create a minimal snapshot.
        let snapshot_params = SnapshotParams::new(
            params.conversation_id,
            params.session_id,
            SequenceRange::new(SequenceNumber::new(1), params.sequence_range_end),
            MessageSummary::default(),
            params.snapshot_type,
        );
        let snapshot = ContextWindowSnapshot::new(snapshot_params, &self.clock);

        let mut guard = self
            .snapshots
            .write()
            .map_err(|e| SnapshotError::persistence(std::io::Error::other(e.to_string())))?;

        guard.insert(snapshot.snapshot_id, snapshot.clone());

        Ok(snapshot)
    }

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
