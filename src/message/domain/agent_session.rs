//! Agent session types for tracking agent backend engagement periods.
//!
//! An agent session represents a contiguous period where a single agent backend
//! handles turns within a conversation. Sessions track the turns executed,
//! handoffs that initiated or terminated them, and context snapshots.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::context_snapshot::ContextWindowSnapshot;
use super::{AgentSessionId, ConversationId, HandoffId, SequenceNumber, TurnId};

/// Represents a contiguous period where a single agent handles a conversation.
///
/// Sessions are created when an agent begins processing a conversation and end
/// via handoff to another agent or normal completion.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::{
///     AgentSession, AgentSessionId, AgentSessionState, ConversationId, SequenceNumber,
/// };
/// use mockable::DefaultClock;
///
/// let clock = DefaultClock;
/// let session = AgentSession::new(
///     ConversationId::new(),
///     "claude-code",
///     SequenceNumber::new(1),
///     &clock,
/// );
/// assert_eq!(session.state, AgentSessionState::Active);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique identifier for this session.
    pub session_id: AgentSessionId,

    /// The conversation this session belongs to.
    pub conversation_id: ConversationId,

    /// The agent backend handling this session.
    pub agent_backend: String,

    /// Sequence number when this session started.
    pub start_sequence: SequenceNumber,

    /// Sequence number when this session ended (None if active).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_sequence: Option<SequenceNumber>,

    /// Turns executed within this session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub turn_ids: Vec<TurnId>,

    /// Handoff that initiated this session (None for initial session).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initiated_by_handoff: Option<HandoffId>,

    /// Handoff that ended this session (None if still active or completed normally).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminated_by_handoff: Option<HandoffId>,

    /// Context snapshots captured during this session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_snapshots: Vec<ContextWindowSnapshot>,

    /// When the session started.
    pub started_at: DateTime<Utc>,

    /// When the session ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,

    /// Session state.
    pub state: AgentSessionState,
}

/// Parameters for creating a session initiated by a handoff.
#[derive(Debug, Clone)]
pub struct HandoffSessionParams {
    /// The conversation this session belongs to.
    pub conversation_id: ConversationId,
    /// The agent backend handling this session.
    pub agent_backend: String,
    /// Sequence number when this session started.
    pub start_sequence: SequenceNumber,
    /// The handoff that initiated this session.
    pub handoff_id: HandoffId,
}

impl HandoffSessionParams {
    /// Creates new handoff session parameters.
    #[must_use]
    pub fn new(
        conversation_id: ConversationId,
        agent_backend: impl Into<String>,
        start_sequence: SequenceNumber,
        handoff_id: HandoffId,
    ) -> Self {
        Self {
            conversation_id,
            agent_backend: agent_backend.into(),
            start_sequence,
            handoff_id,
        }
    }
}

impl AgentSession {
    /// Creates a new active agent session.
    #[must_use]
    pub fn new(
        conversation_id: ConversationId,
        agent_backend: impl Into<String>,
        start_sequence: SequenceNumber,
        clock: &impl mockable::Clock,
    ) -> Self {
        Self {
            session_id: AgentSessionId::new(),
            conversation_id,
            agent_backend: agent_backend.into(),
            start_sequence,
            end_sequence: None,
            turn_ids: Vec::new(),
            initiated_by_handoff: None,
            terminated_by_handoff: None,
            context_snapshots: Vec::new(),
            started_at: clock.utc(),
            ended_at: None,
            state: AgentSessionState::Active,
        }
    }

    /// Creates a session that was initiated by a handoff.
    #[must_use]
    pub fn from_handoff(params: HandoffSessionParams, clock: &impl mockable::Clock) -> Self {
        Self {
            initiated_by_handoff: Some(params.handoff_id),
            ..Self::new(params.conversation_id, params.agent_backend, params.start_sequence, clock)
        }
    }

    /// Records a turn as executed within this session.
    #[must_use]
    pub fn with_turn(mut self, turn_id: TurnId) -> Self {
        self.turn_ids.push(turn_id);
        self
    }

    /// Records a turn as executed (mutable version).
    pub fn record_turn(&mut self, turn_id: TurnId) {
        self.turn_ids.push(turn_id);
    }

    /// Adds a context snapshot to this session.
    #[must_use]
    pub fn with_snapshot(mut self, snapshot: ContextWindowSnapshot) -> Self {
        self.context_snapshots.push(snapshot);
        self
    }

    /// Adds a context snapshot (mutable version).
    pub fn add_snapshot(&mut self, snapshot: ContextWindowSnapshot) {
        self.context_snapshots.push(snapshot);
    }

    /// Pauses the session.
    pub const fn pause(&mut self) {
        self.state = AgentSessionState::Paused;
    }

    /// Resumes a paused session.
    pub const fn resume(&mut self) {
        self.state = AgentSessionState::Active;
    }

    /// Ends the session via handoff to another agent.
    pub fn handoff(
        &mut self,
        end_sequence: SequenceNumber,
        handoff_id: HandoffId,
        clock: &impl mockable::Clock,
    ) {
        self.end_sequence = Some(end_sequence);
        self.terminated_by_handoff = Some(handoff_id);
        self.ended_at = Some(clock.utc());
        self.state = AgentSessionState::HandedOff;
    }

    /// Completes the session normally.
    pub fn complete(&mut self, end_sequence: SequenceNumber, clock: &impl mockable::Clock) {
        self.end_sequence = Some(end_sequence);
        self.ended_at = Some(clock.utc());
        self.state = AgentSessionState::Completed;
    }

    /// Marks the session as failed.
    pub fn fail(&mut self, end_sequence: SequenceNumber, clock: &impl mockable::Clock) {
        self.end_sequence = Some(end_sequence);
        self.ended_at = Some(clock.utc());
        self.state = AgentSessionState::Failed;
    }

    /// Returns `true` if the session is in a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Returns `true` if the session is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.state, AgentSessionState::Active)
    }

    /// Returns the number of turns executed in this session.
    #[must_use]
    pub const fn turn_count(&self) -> usize {
        self.turn_ids.len()
    }
}

/// State of an agent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionState {
    /// Session is active and can process turns.
    Active,

    /// Session is paused (e.g., awaiting user input).
    Paused,

    /// Session ended via handoff to another agent.
    HandedOff,

    /// Session completed normally.
    Completed,

    /// Session failed.
    Failed,
}

impl AgentSessionState {
    /// Returns `true` if this is a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::HandedOff | Self::Completed | Self::Failed)
    }

    /// Returns the state as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::HandedOff => "handed_off",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

impl std::fmt::Display for AgentSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an invalid session state string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseAgentSessionStateError(String);

impl std::fmt::Display for ParseAgentSessionStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid agent session state: '{}'", self.0)
    }
}

impl std::error::Error for ParseAgentSessionStateError {}

impl TryFrom<&str> for AgentSessionState {
    type Error = ParseAgentSessionStateError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "handed_off" => Ok(Self::HandedOff),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(ParseAgentSessionStateError(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockable::DefaultClock;

    #[test]
    fn agent_session_new_is_active() {
        let clock = DefaultClock;
        let session = AgentSession::new(
            ConversationId::new(),
            "claude-code",
            SequenceNumber::new(1),
            &clock,
        );

        assert_eq!(session.state, AgentSessionState::Active);
        assert!(session.is_active());
        assert!(!session.is_terminal());
        assert!(session.end_sequence.is_none());
        assert!(session.ended_at.is_none());
        assert!(session.initiated_by_handoff.is_none());
    }

    #[test]
    fn agent_session_from_handoff() {
        let clock = DefaultClock;
        let handoff_id = HandoffId::new();
        let params = HandoffSessionParams::new(
            ConversationId::new(),
            "opus-agent",
            SequenceNumber::new(10),
            handoff_id,
        );
        let session = AgentSession::from_handoff(params, &clock);

        assert_eq!(session.initiated_by_handoff, Some(handoff_id));
        assert_eq!(session.start_sequence, SequenceNumber::new(10));
    }

    #[test]
    fn agent_session_handoff_terminates() {
        let clock = DefaultClock;
        let mut session = AgentSession::new(
            ConversationId::new(),
            "claude-code",
            SequenceNumber::new(1),
            &clock,
        );

        let handoff_id = HandoffId::new();
        session.handoff(SequenceNumber::new(5), handoff_id, &clock);

        assert_eq!(session.state, AgentSessionState::HandedOff);
        assert!(session.is_terminal());
        assert_eq!(session.end_sequence, Some(SequenceNumber::new(5)));
        assert_eq!(session.terminated_by_handoff, Some(handoff_id));
        assert!(session.ended_at.is_some());
    }

    #[test]
    fn agent_session_record_turns() {
        let clock = DefaultClock;
        let mut session = AgentSession::new(
            ConversationId::new(),
            "claude-code",
            SequenceNumber::new(1),
            &clock,
        );

        session.record_turn(TurnId::new());
        session.record_turn(TurnId::new());

        assert_eq!(session.turn_count(), 2);
    }

    #[test]
    fn agent_session_state_serialisation() {
        assert_eq!(
            serde_json::to_string(&AgentSessionState::Active).expect("serialisation"),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&AgentSessionState::HandedOff).expect("serialisation"),
            "\"handed_off\""
        );
    }
}
