//! Parameter types for the handoff service.

use crate::message::domain::{AgentSessionId, HandoffId, SequenceNumber, TurnId};

/// Parameters for initiating a handoff via the service.
#[derive(Debug, Clone)]
pub struct ServiceInitiateParams<'a> {
    /// The session initiating the handoff.
    pub source_session_id: AgentSessionId,
    /// The agent backend to hand off to.
    pub target_agent: &'a str,
    /// The turn that triggered the handoff.
    pub prior_turn_id: TurnId,
    /// Current sequence number for snapshot.
    pub current_sequence: SequenceNumber,
    /// Optional reason for the handoff.
    pub reason: Option<&'a str>,
}

impl<'a> ServiceInitiateParams<'a> {
    /// Creates new service initiate parameters.
    #[must_use]
    pub const fn new(
        source_session_id: AgentSessionId,
        target_agent: &'a str,
        prior_turn_id: TurnId,
        current_sequence: SequenceNumber,
    ) -> Self {
        Self {
            source_session_id,
            target_agent,
            prior_turn_id,
            current_sequence,
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

/// Parameters for completing a handoff via the service.
#[derive(Debug, Clone, Copy)]
pub struct CompleteHandoffParams {
    /// The handoff to complete.
    pub handoff_id: HandoffId,
    /// The new session created by the target agent.
    pub target_session_id: AgentSessionId,
    /// Starting sequence number for the target session.
    pub start_sequence: SequenceNumber,
}

impl CompleteHandoffParams {
    /// Creates new completion parameters.
    #[must_use]
    pub const fn new(
        handoff_id: HandoffId,
        target_session_id: AgentSessionId,
        start_sequence: SequenceNumber,
    ) -> Self {
        Self {
            handoff_id,
            target_session_id,
            start_sequence,
        }
    }
}
