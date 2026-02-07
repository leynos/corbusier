//! Port for agent session persistence.
//!
//! Defines the abstract interface for storing and retrieving agent sessions,
//! enabling different persistence implementations.

use crate::message::domain::{AgentSession, AgentSessionId, ConversationId};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for session repository operations.
pub type SessionResult<T> = Result<T, SessionError>;

/// Port for agent session persistence operations.
///
/// Implementations provide storage for agent session lifecycle tracking,
/// including creation, updates, and queries.
///
/// # Implementation Notes
///
/// Implementations must ensure:
/// - Session IDs are unique across the entire system
/// - Only one active session per conversation at any time
/// - Sessions are mutable during their lifecycle
/// - Concurrent access is handled safely
#[async_trait]
pub trait AgentSessionRepository: Send + Sync {
    /// Stores a new agent session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` if:
    /// - A session with the same ID already exists
    /// - The database connection fails
    async fn store(&self, session: &AgentSession) -> SessionResult<()>;

    /// Updates an existing session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError::NotFound` if the session does not exist.
    async fn update(&self, session: &AgentSession) -> SessionResult<()>;

    /// Retrieves a session by its ID.
    ///
    /// Returns `None` if the session does not exist.
    async fn find_by_id(&self, id: AgentSessionId) -> SessionResult<Option<AgentSession>>;

    /// Finds the active session for a conversation.
    ///
    /// Returns `None` if no active session exists.
    async fn find_active_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> SessionResult<Option<AgentSession>>;

    /// Lists all sessions for a conversation in chronological order.
    ///
    /// Returns an empty vector if no sessions exist.
    async fn find_by_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> SessionResult<Vec<AgentSession>>;
}

/// Errors that can occur during session repository operations.
#[derive(Debug, Clone, Error)]
pub enum SessionError {
    /// Session not found.
    #[error("session not found: {0}")]
    NotFound(AgentSessionId),

    /// Duplicate session ID.
    #[error("duplicate session: {0}")]
    Duplicate(AgentSessionId),

    /// Conversation not found (when validating foreign key).
    #[error("conversation not found: {0}")]
    ConversationNotFound(ConversationId),

    /// Database or connection error.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl SessionError {
    /// Creates a persistence error from any error type.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
