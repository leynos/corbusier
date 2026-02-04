//! Port for agent handoff operations.
//!
//! Defines the abstract interface for initiating and completing handoffs
//! between agent backends while preserving context.

use crate::message::domain::{
    AgentSession, AgentSessionId, ConversationId, HandoffId, HandoffMetadata, HandoffStatus, TurnId,
};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for handoff operations.
pub type HandoffResult<T> = Result<T, HandoffError>;

/// Parameters for initiating a handoff.
#[derive(Debug, Clone)]
pub struct InitiateHandoffParams<'a> {
    /// The conversation being handed off.
    pub conversation_id: ConversationId,
    /// The current agent session.
    pub source_session: &'a AgentSession,
    /// The agent backend to hand off to.
    pub target_agent: &'a str,
    /// The turn that triggered the handoff.
    pub prior_turn_id: TurnId,
    /// Optional reason for the handoff.
    pub reason: Option<&'a str>,
}

impl<'a> InitiateHandoffParams<'a> {
    /// Creates new initiate handoff parameters.
    #[must_use]
    pub const fn new(
        conversation_id: ConversationId,
        source_session: &'a AgentSession,
        target_agent: &'a str,
        prior_turn_id: TurnId,
    ) -> Self {
        Self {
            conversation_id,
            source_session,
            target_agent,
            prior_turn_id,
            reason: None,
        }
    }

    /// Sets the reason for the handoff.
    #[must_use]
    pub const fn with_reason(mut self, reason: &'a str) -> Self {
        self.reason = Some(reason);
        self
    }
}

/// Port for agent handoff operations.
///
/// Implementations coordinate context transfer between agent backends
/// and ensure no context is lost during transitions.
#[async_trait]
pub trait AgentHandoffPort: Send + Sync {
    /// Initiates a handoff from the current agent to a target agent.
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if the handoff could not be initiated.
    async fn initiate_handoff(
        &self,
        params: InitiateHandoffParams<'_>,
    ) -> HandoffResult<HandoffMetadata>;

    /// Completes a handoff after the target agent acknowledges.
    ///
    /// Creates a new agent session for the target agent and captures
    /// context snapshots for both sessions.
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if the handoff cannot be completed or persisted.
    async fn complete_handoff(
        &self,
        handoff_id: HandoffId,
        target_session_id: AgentSessionId,
    ) -> HandoffResult<HandoffMetadata>;

    /// Cancels a pending handoff.
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if the handoff cannot be cancelled or persisted.
    async fn cancel_handoff(
        &self,
        handoff_id: HandoffId,
        reason: Option<&str>,
    ) -> HandoffResult<()>;

    /// Retrieves handoff metadata by ID.
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if lookup fails.
    async fn find_handoff(&self, handoff_id: HandoffId) -> HandoffResult<Option<HandoffMetadata>>;

    /// Lists all handoffs for a conversation.
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if the list cannot be retrieved.
    async fn list_handoffs_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> HandoffResult<Vec<HandoffMetadata>>;
}

/// Errors that can occur during handoff operations.
#[derive(Debug, Clone, Error)]
pub enum HandoffError {
    /// Handoff not found.
    #[error("handoff not found: {0}")]
    NotFound(HandoffId),

    /// Invalid handoff state transition.
    #[error("invalid handoff state transition from {from} to {to}")]
    InvalidStateTransition {
        /// The current state.
        from: HandoffStatus,
        /// The attempted target state.
        to: HandoffStatus,
    },

    /// Agent session not found.
    #[error("agent session not found: {0}")]
    SessionNotFound(AgentSessionId),

    /// Conversation not found.
    #[error("conversation not found: {0}")]
    ConversationNotFound(ConversationId),

    /// Prior turn not found.
    #[error("prior turn not found: {0}")]
    PriorTurnNotFound(TurnId),

    /// Context snapshot capture failed.
    #[error("context snapshot failed: {0}")]
    SnapshotFailed(String),

    /// Agent session update failed.
    #[error("session update failed: {0}")]
    SessionUpdateFailed(String),

    /// Database or persistence error.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl HandoffError {
    /// Creates a persistence error from any error type.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }

    /// Creates an invalid state transition error.
    #[must_use]
    pub const fn invalid_transition(from: HandoffStatus, to: HandoffStatus) -> Self {
        Self::InvalidStateTransition { from, to }
    }
}
