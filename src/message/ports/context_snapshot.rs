//! Port for context window snapshot operations.
//!
//! Defines the abstract interface for capturing and retrieving context window
//! snapshots, enabling audit and reconstruction of agent session state.

use crate::message::domain::{AgentSessionId, ContextWindowSnapshot, ConversationId};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Result type for snapshot operations.
pub type SnapshotResult<T> = Result<T, SnapshotError>;

/// Port for context window snapshot operations.
///
/// Implementations store and retrieve snapshots of the context window
/// at various points during an agent session.
#[async_trait]
pub trait ContextSnapshotPort: Send + Sync {
    /// Stores a pre-built context snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError::Duplicate`] if a snapshot with the same ID
    /// already exists, or [`SnapshotError::Persistence`] if storage fails.
    async fn store_snapshot(&self, snapshot: &ContextWindowSnapshot) -> SnapshotResult<()>;

    /// Retrieves a snapshot by its ID.
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError::Persistence`] if the lookup fails.
    async fn find_by_id(&self, snapshot_id: Uuid) -> SnapshotResult<Option<ContextWindowSnapshot>>;

    /// Retrieves snapshots for a session.
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError::Persistence`] if retrieval fails.
    async fn find_snapshots_for_session(
        &self,
        session_id: AgentSessionId,
    ) -> SnapshotResult<Vec<ContextWindowSnapshot>>;

    /// Retrieves the most recent snapshot for a conversation.
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError::Persistence`] if retrieval fails.
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
