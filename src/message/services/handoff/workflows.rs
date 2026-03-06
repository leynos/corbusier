//! Handoff workflow orchestration.
//!
//! Contains the [`HandoffService`] which coordinates the lifecycle of agent
//! handoffs, ensuring context is preserved and proper audit trails are
//! maintained.

use std::sync::Arc;

use mockable::Clock;

use super::params::{CompleteHandoffParams, ServiceInitiateParams};
use crate::context::RequestContext;
use crate::message::{
    domain::{
        AgentSession, ContextWindowSnapshot, ConversationId, HandoffId, HandoffMetadata,
        HandoffSessionParams, MessageSummary, SequenceRange, SnapshotParams, SnapshotType,
    },
    ports::{
        agent_session::{AgentSessionRepository, SessionResult},
        context_snapshot::ContextSnapshotPort,
        handoff::{AgentHandoffPort, HandoffError, HandoffResult, InitiateHandoffParams},
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
/// use corbusier::context::RequestContext;
/// use corbusier::message::services::{
///     HandoffService, ServiceInitiateParams, CompleteHandoffParams,
/// };
///
/// let service = HandoffService::new(
///     session_repo,
///     handoff_adapter,
///     snapshot_adapter,
///     clock,
/// );
/// let ctx = RequestContext::new(tenant_id, correlation_id, user_id, session_id);
///
/// // Initiate handoff
/// let params = ServiceInitiateParams::new(
///     source_session_id,
///     "target-agent",
///     prior_turn_id,
///     current_sequence,
/// ).with_reason("task too complex");
/// let handoff = service.initiate(&ctx, params).await?;
///
/// // Complete handoff when target agent starts
/// let complete_params = CompleteHandoffParams::new(
///     handoff.handoff_id,
///     target_session_id,
///     start_sequence,
/// );
/// let completed = service.complete(&ctx, complete_params).await?;
/// ```
#[derive(Clone)]
pub struct HandoffService<S, H, C, K>
where
    S: AgentSessionRepository,
    H: AgentHandoffPort,
    C: ContextSnapshotPort,
    K: Clock + Send + Sync,
{
    session_repo: Arc<S>,
    handoff_adapter: Arc<H>,
    snapshot_adapter: Arc<C>,
    clock: Arc<K>,
}

impl<S, H, C, K> HandoffService<S, H, C, K>
where
    S: AgentSessionRepository,
    H: AgentHandoffPort,
    C: ContextSnapshotPort,
    K: Clock + Send + Sync,
{
    /// Creates a new handoff service.
    pub const fn new(
        session_repo: Arc<S>,
        handoff_adapter: Arc<H>,
        snapshot_adapter: Arc<C>,
        clock: Arc<K>,
    ) -> Self {
        Self {
            session_repo,
            handoff_adapter,
            snapshot_adapter,
            clock,
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
    /// # Errors
    ///
    /// Returns `HandoffError` if:
    /// - Source session not found
    /// - Source session is not active
    /// - Handoff creation fails
    /// - Source session update fails
    pub async fn initiate(
        &self,
        ctx: &RequestContext,
        params: ServiceInitiateParams<'_>,
    ) -> HandoffResult<HandoffMetadata> {
        // Find and validate source session
        let mut source_session = self
            .session_repo
            .find_by_id(ctx, params.source_session_id)
            .await
            .map_err(|_| HandoffError::SessionNotFound(params.source_session_id))?
            .ok_or(HandoffError::SessionNotFound(params.source_session_id))?;

        if !source_session.is_active() {
            return Err(HandoffError::InvalidStateTransition {
                from: source_session.state.into(),
                to: crate::message::domain::HandoffStatus::Initiated,
            });
        }

        // Capture context snapshot before handoff
        let snapshot = self.build_snapshot(SnapshotParams {
            conversation_id: source_session.conversation_id,
            session_id: params.source_session_id,
            sequence_range: SequenceRange::new(
                source_session.start_sequence,
                params.current_sequence,
            ),
            message_summary: MessageSummary::default(),
            snapshot_type: SnapshotType::HandoffInitiated,
        });
        self.snapshot_adapter
            .store_snapshot(ctx, &snapshot)
            .await
            .map_err(|e| HandoffError::SnapshotFailed(e.to_string()))?;

        // Initiate the handoff
        let mut handoff_params = InitiateHandoffParams::new(
            source_session.conversation_id,
            &source_session,
            params.target_agent,
            params.prior_turn_id,
        );
        if let Some(r) = params.reason {
            handoff_params = handoff_params.with_reason(r);
        }
        let handoff = self
            .handoff_adapter
            .initiate_handoff(ctx, handoff_params)
            .await?;

        // Update source session state
        source_session.handoff(
            params.current_sequence,
            handoff.handoff_id,
            self.clock.as_ref(),
        );

        self.session_repo
            .update(ctx, &source_session)
            .await
            .map_err(|e| HandoffError::SessionUpdateFailed(e.to_string()))?;

        Ok(handoff)
    }

    /// Completes a handoff by recording the target session and marking complete.
    ///
    /// This method:
    /// 1. Validates the handoff exists and is in correct state
    /// 2. Creates a context snapshot for the new session start
    /// 3. Completes the handoff record
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if:
    /// - Handoff not found
    /// - Handoff is not in `Initiated` or `Accepted` state
    /// - Target session not found
    pub async fn complete(
        &self,
        ctx: &RequestContext,
        params: CompleteHandoffParams,
    ) -> HandoffResult<HandoffMetadata> {
        let CompleteHandoffParams {
            handoff_id,
            target_session_id,
            start_sequence,
        } = params;
        // Verify the handoff exists and is in valid state
        let handoff = self
            .handoff_adapter
            .find_handoff(ctx, handoff_id)
            .await?
            .ok_or(HandoffError::NotFound(handoff_id))?;

        // Find the target session to get conversation_id
        let target_session = self
            .session_repo
            .find_by_id(ctx, target_session_id)
            .await
            .map_err(|_| HandoffError::SessionNotFound(target_session_id))?
            .ok_or(HandoffError::SessionNotFound(target_session_id))?;

        // Verify handoff source and target sessions share the same conversation
        let source_session = self
            .session_repo
            .find_by_id(ctx, handoff.source_session_id)
            .await
            .map_err(|_| HandoffError::SessionNotFound(handoff.source_session_id))?
            .ok_or(HandoffError::SessionNotFound(handoff.source_session_id))?;

        if source_session.conversation_id != target_session.conversation_id {
            return Err(HandoffError::ConversationMismatch {
                source_conversation: source_session.conversation_id,
                target_conversation: target_session.conversation_id,
            });
        }

        // Capture session start snapshot for target
        let snapshot = self.build_snapshot(SnapshotParams {
            conversation_id: target_session.conversation_id,
            session_id: target_session_id,
            sequence_range: SequenceRange::new(start_sequence, start_sequence),
            message_summary: MessageSummary::default(),
            snapshot_type: SnapshotType::SessionStart,
        });
        self.snapshot_adapter
            .store_snapshot(ctx, &snapshot)
            .await
            .map_err(|e| HandoffError::SnapshotFailed(e.to_string()))?;

        // Complete the handoff
        let completed = self
            .handoff_adapter
            .complete_handoff(ctx, handoff_id, target_session_id)
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
    /// - Source session update fails
    pub async fn cancel(
        &self,
        ctx: &RequestContext,
        handoff_id: HandoffId,
        reason: Option<&str>,
    ) -> HandoffResult<()> {
        // Find the handoff
        let handoff = self
            .handoff_adapter
            .find_handoff(ctx, handoff_id)
            .await?
            .ok_or(HandoffError::NotFound(handoff_id))?;

        // Revert source session if needed
        if let Some(mut source_session) = self
            .session_repo
            .find_by_id(ctx, handoff.source_session_id)
            .await
            .map_err(|e| HandoffError::SessionUpdateFailed(e.to_string()))?
            && source_session.terminated_by_handoff == Some(handoff_id)
        {
            // Revert to active state
            source_session.state = crate::message::domain::AgentSessionState::Active;
            source_session.terminated_by_handoff = None;
            source_session.end_sequence = None;
            source_session.ended_at = None;

            self.session_repo
                .update(ctx, &source_session)
                .await
                .map_err(|e| HandoffError::SessionUpdateFailed(e.to_string()))?;
        }

        // Cancel the handoff
        self.handoff_adapter
            .cancel_handoff(ctx, handoff_id, reason)
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
    /// # Errors
    ///
    /// Returns `SessionError` if the session could not be stored.
    pub async fn create_target_session(
        &self,
        ctx: &RequestContext,
        params: HandoffSessionParams,
    ) -> SessionResult<AgentSession> {
        let session = AgentSession::from_handoff(params, self.clock.as_ref());

        self.session_repo.store(ctx, &session).await?;

        Ok(session)
    }

    /// Gets the current handoff for a conversation, if any.
    ///
    /// # Errors
    ///
    /// Returns `HandoffError` if the handoff list could not be retrieved.
    pub async fn get_pending_handoff(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HandoffResult<Option<HandoffMetadata>> {
        let handoffs = self
            .handoff_adapter
            .list_handoffs_for_conversation(ctx, conversation_id)
            .await?;

        // Find a non-terminal handoff
        Ok(handoffs.into_iter().find(|h| !h.is_terminal()))
    }

    fn build_snapshot(&self, params: SnapshotParams) -> ContextWindowSnapshot {
        ContextWindowSnapshot::new(params, self.clock.as_ref())
    }
}
