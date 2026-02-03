//! Port for context window snapshot operations.
//!
//! Defines the abstract interface for capturing and retrieving context window
//! snapshots, enabling audit and reconstruction of agent session state.

use crate::message::domain::{
    AgentSessionId, ContextWindowSnapshot, ConversationId, SequenceNumber, SnapshotType,
};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Result type for snapshot operations.
pub type SnapshotResult<T> = Result<T, SnapshotError>;

/// Parameters for capturing a context window snapshot.
#[derive(Debug, Clone)]
pub struct CaptureSnapshotParams {
    /// The conversation to snapshot.
    pub conversation_id: ConversationId,
    /// The agent session this snapshot belongs to.
    pub session_id: AgentSessionId,
    /// The last sequence number to include.
    pub sequence_range_end: SequenceNumber,
    /// The type of snapshot being captured.
    pub snapshot_type: SnapshotType,
}

impl CaptureSnapshotParams {
    /// Creates new capture snapshot parameters.
    #[must_use]
    pub const fn new(
        conversation_id: ConversationId,
        session_id: AgentSessionId,
        sequence_range_end: SequenceNumber,
        snapshot_type: SnapshotType,
    ) -> Self {
        Self {
            conversation_id,
            session_id,
            sequence_range_end,
            snapshot_type,
        }
    }
}

/// Port for context window snapshot operations.
///
/// Implementations capture and store snapshots of the context window
/// at various points during an agent session.
#[async_trait]
pub trait ContextSnapshotPort: Send + Sync {
    /// Captures a context window snapshot for an agent session.
    ///
    /// The snapshot includes all messages from the session start up to
    /// the specified sequence number, along with message summaries and
    /// visible tool calls.
    async fn capture_snapshot(
        &self,
        params: CaptureSnapshotParams,
    ) -> SnapshotResult<ContextWindowSnapshot>;

    /// Stores a pre-built context snapshot.
    async fn store_snapshot(&self, snapshot: &ContextWindowSnapshot) -> SnapshotResult<()>;

    /// Retrieves a snapshot by its ID.
    async fn find_by_id(&self, snapshot_id: Uuid) -> SnapshotResult<Option<ContextWindowSnapshot>>;

    /// Retrieves snapshots for a session.
    async fn find_snapshots_for_session(
        &self,
        session_id: AgentSessionId,
    ) -> SnapshotResult<Vec<ContextWindowSnapshot>>;

    /// Retrieves the most recent snapshot for a conversation.
    async fn find_latest_snapshot(
        &self,
        conversation_id: ConversationId,
    ) -> SnapshotResult<Option<ContextWindowSnapshot>>;
}

/// Errors that can occur during snapshot operations.
#[derive(Debug, Clone, Error)]
pub enum SnapshotError {
    /// Snapshot not found.
    #[error("snapshot not found: {0}")]
    NotFound(Uuid),

    /// Duplicate snapshot ID.
    #[error("duplicate snapshot: {0}")]
    Duplicate(Uuid),

    /// Session not found.
    #[error("session not found: {0}")]
    SessionNotFound(AgentSessionId),

    /// Conversation not found.
    #[error("conversation not found: {0}")]
    ConversationNotFound(ConversationId),

    /// No messages in the specified range.
    #[error("no messages in range")]
    EmptyRange,

    /// Database or persistence error.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl SnapshotError {
    /// Creates a persistence error from any error type.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
