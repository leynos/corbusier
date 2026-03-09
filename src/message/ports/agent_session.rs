//! Port for agent session persistence.
//!
//! Defines the abstract interface for storing and retrieving agent sessions,
//! enabling different persistence implementations.

use crate::context::RequestContext;
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
/// - **Tenant isolation is not yet enforced here.** The `ctx`
///   parameter (`_ctx` in current adapters) carries
///   [`RequestContext::tenant_id`](crate::context::RequestContext)
///   but adapter implementations do not filter by tenant until
///   Row-Level Security (RLS) or adapter-level filtering is wired
///   (planned for milestone 1.5.3).  Callers must not rely on this
///   trait to enforce tenant boundaries at present
#[async_trait]
pub trait AgentSessionRepository: Send + Sync {
    /// Stores a new agent session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` if:
    /// - A session with the same ID already exists ([`SessionError::Duplicate`])
    /// - An active session already exists for the conversation
    ///   ([`SessionError::ActiveSessionExists`])
    /// - The database connection fails ([`SessionError::Persistence`])
    async fn store(&self, ctx: &RequestContext, session: &AgentSession) -> SessionResult<()>;

    /// Updates an existing session.
    ///
    /// # Errors
    ///
    /// Returns `SessionError` if:
    /// - The session does not exist ([`SessionError::NotFound`])
    /// - The update would create a second active session for the
    ///   conversation ([`SessionError::ActiveSessionExists`])
    /// - The database connection fails ([`SessionError::Persistence`])
    async fn update(&self, ctx: &RequestContext, session: &AgentSession) -> SessionResult<()>;

    /// Retrieves a session by its ID.
    ///
    /// Returns `None` if the session does not exist.
    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        id: AgentSessionId,
    ) -> SessionResult<Option<AgentSession>>;

    /// Finds the active session for a conversation.
    ///
    /// Returns `None` if no active session exists.
    async fn find_active_for_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> SessionResult<Option<AgentSession>>;

    /// Lists all sessions for a conversation in chronological order.
    ///
    /// Returns an empty vector if no sessions exist.
    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
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

    /// An active session already exists for the conversation.
    #[error("active session already exists for conversation: {0}")]
    ActiveSessionExists(ConversationId),

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
