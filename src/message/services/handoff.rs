//! Handoff service for orchestrating agent transitions.
//!
//! The `HandoffService` coordinates the lifecycle of agent handoffs,
//! ensuring context is preserved and proper audit trails are maintained.

use std::sync::Arc;

use crate::message::{
    domain::{
        AgentSession, AgentSessionId, ConversationId, HandoffId, HandoffMetadata, SequenceNumber,
        SnapshotType, TurnId,
    },
    ports::{
        agent_session::{AgentSessionRepository, SessionResult},
        context_snapshot::ContextSnapshotPort,
        handoff::{AgentHandoffPort, HandoffError, HandoffResult},
    },
};

/// Service for coordinating agent handoffs with context preservation.
///
/// Orchestrates the complete handoff workflow:
/// 1. Captures context snapshot from source session
/// 2. Initiates the handoff record
/// 3. Creates the target session (from handoff)
/// 4. Captures context snapshot for target session
/// 5. Completes the handoff
///
/// # Example
///
/// ```ignore
/// use corbusier::message::services::HandoffService;
///
/// let service = HandoffService::new(
///     session_repo,
///     handoff_adapter,
///     snapshot_adapter,
/// );
///
/// // Initiate handoff
/// let handoff = service.initiate(
///     conversation_id,
///     source_session_id,
///     "target-agent",
///     prior_turn_id,
///     current_sequence,
///     Some("task too complex"),
/// ).await?;
///
/// // Complete handoff when target agent starts
/// let completed = service.complete(
///     handoff.handoff_id,
///     target_session_id,
///     start_sequence,
/// ).await?;
/// ```
#[derive(Debug, Clone)]
pub struct HandoffService<S, H, C>
where
    S: AgentSessionRepository,
    H: AgentHandoffPort,
    C: ContextSnapshotPort,
{
    session_repo: Arc<S>,
    handoff_adapter: Arc<H>,
    snapshot_adapter: Arc<C>,
}

impl<S, H, C> HandoffService<S, H, C>
where
    S: AgentSessionRepository,
    H: AgentHandoffPort,
    C: ContextSnapshotPort,
{
    /// Creates a new handoff service.
    pub fn new(session_repo: Arc<S>, handoff_adapter: Arc<H>, snapshot_adapter: Arc<C>) -> Self {
        Self {
            session_repo,
            handoff_adapter,
            snapshot_adapter,
        }
    }

    /// Initiates a handoff from the current active session to a target agent.
    ///
    /// This method:
    /// 1. Finds and validates the source session
    /// 2. Captures a context snapshot of the current state
    /// 3. Creates the handoff record
    /// 4. Updates the source session state
    ///
    /// # Parameters
    ///
    /// - `conversation_id`: The conversation being handed off
    /// - `source_session_id`: The session initiating the handoff
    /// - `target_agent`: The agent backend to hand off to
    /// - `prior_turn_id`: The turn that triggered the handoff
    /// - `current_sequence`: Current sequence number for snapshot
    /// - `reason`: Optional reason for the handoff
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if:
    /// - Source session not found
    /// - Source session is not active
    /// - Handoff creation fails
    pub async fn initiate(
        &self,
        conversation_id: ConversationId,
        source_session_id: AgentSessionId,
        target_agent: &str,
        prior_turn_id: TurnId,
        current_sequence: SequenceNumber,
        reason: Option<&str>,
    ) -> HandoffResult<HandoffMetadata> {
        // Find and validate source session
        let mut source_session = self
            .session_repo
            .find_by_id(source_session_id)
            .await
            .map_err(|_| HandoffError::SessionNotFound(source_session_id))?
            .ok_or(HandoffError::SessionNotFound(source_session_id))?;

        if !source_session.is_active() {
            return Err(HandoffError::InvalidStateTransition {
                from: source_session.state.into(),
                to: crate::message::domain::HandoffStatus::Initiated,
            });
        }

        // Capture context snapshot before handoff
        let _snapshot = self
            .snapshot_adapter
            .capture_snapshot(
                conversation_id,
                source_session_id,
                current_sequence,
                SnapshotType::HandoffInitiated,
            )
            .await
            .map_err(|e| HandoffError::SnapshotFailed(e.to_string()))?;

        // Initiate the handoff
        let handoff = self
            .handoff_adapter
            .initiate_handoff(
                conversation_id,
                &source_session,
                target_agent,
                prior_turn_id,
                reason,
            )
            .await?;

        // Update source session state
        let clock = mockable::DefaultClock;
        source_session.handoff(current_sequence, handoff.handoff_id, &clock);

        self.session_repo
            .update(&source_session)
            .await
            .map_err(|e| HandoffError::SnapshotFailed(e.to_string()))?;

        Ok(handoff)
    }

    /// Completes a handoff by recording the target session and marking complete.
    ///
    /// This method:
    /// 1. Validates the handoff exists and is in correct state
    /// 2. Creates a context snapshot for the new session start
    /// 3. Completes the handoff record
    ///
    /// # Parameters
    ///
    /// - `handoff_id`: The handoff to complete
    /// - `target_session_id`: The new session created by the target agent
    /// - `start_sequence`: Starting sequence number for the target session
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if:
    /// - Handoff not found
    /// - Handoff is not in `Initiated` or `Accepted` state
    /// - Target session not found
    pub async fn complete(
        &self,
        handoff_id: HandoffId,
        target_session_id: AgentSessionId,
        start_sequence: SequenceNumber,
    ) -> HandoffResult<HandoffMetadata> {
        // Verify the handoff exists and is in valid state
        let _handoff = self
            .handoff_adapter
            .find_handoff(handoff_id)
            .await?
            .ok_or(HandoffError::NotFound(handoff_id))?;

        // Find the target session to get conversation_id
        let target_session = self
            .session_repo
            .find_by_id(target_session_id)
            .await
            .map_err(|_| HandoffError::SessionNotFound(target_session_id))?
            .ok_or(HandoffError::SessionNotFound(target_session_id))?;

        // Capture session start snapshot for target
        let _snapshot = self
            .snapshot_adapter
            .capture_snapshot(
                target_session.conversation_id,
                target_session_id,
                start_sequence,
                SnapshotType::SessionStart,
            )
            .await
            .map_err(|e| HandoffError::SnapshotFailed(e.to_string()))?;

        // Complete the handoff
        let completed = self
            .handoff_adapter
            .complete_handoff(handoff_id, target_session_id)
            .await?;

        Ok(completed)
    }

    /// Cancels a pending handoff.
    ///
    /// Reverts the source session to active state if it was marked as handed off.
    ///
    /// # Parameters
    ///
    /// - `handoff_id`: The handoff to cancel
    /// - `reason`: Optional reason for cancellation
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if:
    /// - Handoff not found
    /// - Handoff is already in a terminal state
    pub async fn cancel(&self, handoff_id: HandoffId, reason: Option<&str>) -> HandoffResult<()> {
        // Find the handoff
        let handoff = self
            .handoff_adapter
            .find_handoff(handoff_id)
            .await?
            .ok_or(HandoffError::NotFound(handoff_id))?;

        // Revert source session if needed
        if let Ok(Some(mut source_session)) = self
            .session_repo
            .find_by_id(handoff.source_session_id)
            .await
        {
            if source_session.terminated_by_handoff == Some(handoff_id) {
                // Revert to active state
                source_session.state = crate::message::domain::AgentSessionState::Active;
                source_session.terminated_by_handoff = None;
                source_session.end_sequence = None;
                source_session.ended_at = None;

                let _ = self.session_repo.update(&source_session).await;
            }
        }

        // Cancel the handoff
        self.handoff_adapter
            .cancel_handoff(handoff_id, reason)
            .await
    }

    /// Creates a new session for the target agent during handoff acceptance.
    ///
    /// This is called by the target agent when it accepts the handoff
    /// and needs to create its session.
    ///
    /// # Parameters
    ///
    /// - `conversation_id`: The conversation being handed off
    /// - `agent_backend`: The target agent backend identifier
    /// - `start_sequence`: Starting sequence number
    /// - `handoff_id`: The handoff that initiated this session
    ///
    /// # Returns
    ///
    /// The newly created agent session.
    pub async fn create_target_session(
        &self,
        conversation_id: ConversationId,
        agent_backend: &str,
        start_sequence: SequenceNumber,
        handoff_id: HandoffId,
    ) -> SessionResult<AgentSession> {
        let clock = mockable::DefaultClock;
        let session = AgentSession::from_handoff(
            conversation_id,
            agent_backend,
            start_sequence,
            handoff_id,
            &clock,
        );

        self.session_repo.store(&session).await?;

        Ok(session)
    }

    /// Gets the current handoff for a conversation, if any.
    pub async fn get_pending_handoff(
        &self,
        conversation_id: ConversationId,
    ) -> HandoffResult<Option<HandoffMetadata>> {
        let handoffs = self
            .handoff_adapter
            .list_handoffs_for_conversation(conversation_id)
            .await?;

        // Find a non-terminal handoff
        Ok(handoffs.into_iter().find(|h| !h.is_terminal()))
    }
}

// Conversion helper for session state to handoff status
impl From<crate::message::domain::AgentSessionState> for crate::message::domain::HandoffStatus {
    fn from(state: crate::message::domain::AgentSessionState) -> Self {
        match state {
            crate::message::domain::AgentSessionState::Active => Self::Initiated,
            crate::message::domain::AgentSessionState::Paused => Self::Initiated,
            crate::message::domain::AgentSessionState::HandedOff => Self::Completed,
            crate::message::domain::AgentSessionState::Completed => Self::Completed,
            crate::message::domain::AgentSessionState::Failed => Self::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::adapters::memory::{
        InMemoryAgentSessionRepository, InMemoryContextSnapshotAdapter, InMemoryHandoffAdapter,
    };
    use mockable::DefaultClock;

    fn create_service() -> HandoffService<
        InMemoryAgentSessionRepository,
        InMemoryHandoffAdapter<DefaultClock>,
        InMemoryContextSnapshotAdapter<DefaultClock>,
    > {
        HandoffService::new(
            Arc::new(InMemoryAgentSessionRepository::new()),
            Arc::new(InMemoryHandoffAdapter::new(DefaultClock)),
            Arc::new(InMemoryContextSnapshotAdapter::new(DefaultClock)),
        )
    }

    #[tokio::test]
    async fn initiate_handoff_requires_active_session() {
        let service = create_service();
        let conversation_id = ConversationId::new();
        let session_id = AgentSessionId::new();

        // No session exists, should fail
        let result = service
            .initiate(
                conversation_id,
                session_id,
                "target-agent",
                TurnId::new(),
                SequenceNumber::new(5),
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            HandoffError::SessionNotFound(_)
        ));
    }

    #[tokio::test]
    async fn create_target_session_stores_session() {
        let service = create_service();
        let conversation_id = ConversationId::new();
        let handoff_id = HandoffId::new();

        let session = service
            .create_target_session(
                conversation_id,
                "target-agent",
                SequenceNumber::new(10),
                handoff_id,
            )
            .await
            .expect("should create session");

        assert_eq!(session.conversation_id, conversation_id);
        assert_eq!(session.initiated_by_handoff, Some(handoff_id));
        assert_eq!(session.agent_backend, "target-agent");

        // Verify it was stored
        let found = service
            .session_repo
            .find_by_id(session.session_id)
            .await
            .expect("should find")
            .expect("session should exist");

        assert_eq!(found.session_id, session.session_id);
    }
}
