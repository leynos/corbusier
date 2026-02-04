//! Agent handoff types for tracking context preservation across agent transitions.
//!
//! When control of a conversation transfers from one agent backend to another,
//! handoff metadata captures the prior turn reference, tool calls that led to
//! the handoff, and the target agent. This enables complete audit trails and
//! context reconstruction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{AgentSessionId, HandoffId, MessageId, SequenceNumber, TurnId};

/// Metadata capturing the context at the point of an agent handoff.
///
/// Stored as part of `MessageMetadata` to ensure handoffs are fully auditable
/// and context is preserved for the target agent.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::{
///     AgentSessionId, HandoffMetadata, HandoffParams, HandoffStatus, TurnId,
/// };
/// use mockable::DefaultClock;
///
/// let clock = DefaultClock;
/// let params = HandoffParams::new(
///     AgentSessionId::new(),
///     TurnId::new(),
///     "claude-code",
///     "opus-agent",
/// );
/// let handoff = HandoffMetadata::new(params, &clock);
/// assert_eq!(handoff.status, HandoffStatus::Initiated);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoffMetadata {
    /// Unique identifier for this handoff event.
    pub handoff_id: HandoffId,

    /// The agent session being handed off from.
    pub source_session_id: AgentSessionId,

    /// The agent session being handed off to (populated after handoff completes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_session_id: Option<AgentSessionId>,

    /// The turn ID that triggered this handoff.
    pub prior_turn_id: TurnId,

    /// References to tool calls that led to the handoff decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggering_tool_calls: Vec<ToolCallReference>,

    /// The source agent backend identifier.
    pub source_agent: String,

    /// The target agent backend identifier.
    pub target_agent: String,

    /// Reason or context for the handoff (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// When the handoff was initiated.
    pub initiated_at: DateTime<Utc>,

    /// When the handoff completed (target agent acknowledged).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,

    /// Handoff status.
    pub status: HandoffStatus,
}

/// Parameters for creating handoff metadata.
#[derive(Debug, Clone)]
pub struct HandoffParams {
    /// The agent session being handed off from.
    pub source_session_id: AgentSessionId,
    /// The turn ID that triggered the handoff.
    pub prior_turn_id: TurnId,
    /// The source agent backend identifier.
    pub source_agent: String,
    /// The target agent backend identifier.
    pub target_agent: String,
}

impl HandoffParams {
    /// Creates new handoff parameters.
    #[must_use]
    pub fn new(
        source_session_id: AgentSessionId,
        prior_turn_id: TurnId,
        source_agent: impl Into<String>,
        target_agent: impl Into<String>,
    ) -> Self {
        Self {
            source_session_id,
            prior_turn_id,
            source_agent: source_agent.into(),
            target_agent: target_agent.into(),
        }
    }
}

impl HandoffMetadata {
    /// Creates a new handoff metadata with `Initiated` status.
    #[must_use]
    pub fn new(params: HandoffParams, clock: &impl mockable::Clock) -> Self {
        Self {
            handoff_id: HandoffId::new(),
            source_session_id: params.source_session_id,
            target_session_id: None,
            prior_turn_id: params.prior_turn_id,
            triggering_tool_calls: Vec::new(),
            source_agent: params.source_agent,
            target_agent: params.target_agent,
            reason: None,
            initiated_at: clock.utc(),
            completed_at: None,
            status: HandoffStatus::Initiated,
        }
    }

    /// Adds a tool call reference that contributed to the handoff decision.
    #[must_use]
    pub fn with_triggering_tool_call(mut self, reference: ToolCallReference) -> Self {
        self.triggering_tool_calls.push(reference);
        self
    }

    /// Adds multiple tool call references.
    #[must_use]
    pub fn with_triggering_tool_calls(
        mut self,
        references: impl IntoIterator<Item = ToolCallReference>,
    ) -> Self {
        self.triggering_tool_calls.extend(references);
        self
    }

    /// Sets the reason for the handoff.
    #[must_use]
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Marks the handoff as accepted by the target agent.
    #[must_use]
    pub const fn accept(mut self) -> Self {
        self.status = HandoffStatus::Accepted;
        self
    }

    /// Completes the handoff, recording the target session and completion time.
    #[must_use]
    pub fn complete(
        mut self,
        target_session_id: AgentSessionId,
        clock: &impl mockable::Clock,
    ) -> Self {
        self.target_session_id = Some(target_session_id);
        self.completed_at = Some(clock.utc());
        self.status = HandoffStatus::Completed;
        self
    }

    /// Marks the handoff as failed.
    #[must_use]
    pub const fn fail(mut self) -> Self {
        self.status = HandoffStatus::Failed;
        self
    }

    /// Cancels the handoff, optionally recording a reason.
    #[must_use]
    pub fn cancel(mut self, reason: Option<&str>) -> Self {
        if let Some(reason_text) = reason {
            self.reason = Some(reason_text.to_owned());
        }
        self.status = HandoffStatus::Cancelled;
        self
    }

    /// Returns `true` if the handoff is in a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

/// Reference to a tool call that contributed to a handoff decision.
///
/// Captures the essential identifiers needed to trace back to the original
/// tool call in the conversation history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallReference {
    /// The `call_id` of the referenced tool call.
    pub call_id: String,

    /// The tool name.
    pub tool_name: String,

    /// The message ID containing this tool call.
    pub message_id: MessageId,

    /// The sequence number of the message.
    pub sequence_number: SequenceNumber,
}

impl ToolCallReference {
    /// Creates a new tool call reference.
    #[must_use]
    pub fn new(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        message_id: MessageId,
        sequence_number: SequenceNumber,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            message_id,
            sequence_number,
        }
    }
}

/// Status of a handoff operation.
///
/// Handoffs transition through these states as the target agent acknowledges
/// and takes over the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffStatus {
    /// Handoff initiated, awaiting target agent acknowledgement.
    Initiated,

    /// Target agent has accepted the handoff.
    Accepted,

    /// Handoff completed successfully.
    Completed,

    /// Handoff failed (target agent unavailable or rejected).
    Failed,

    /// Handoff was cancelled before completion.
    Cancelled,
}

impl HandoffStatus {
    /// Returns `true` if this is a terminal status.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Returns the status as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Initiated => "initiated",
            Self::Accepted => "accepted",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for HandoffStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an invalid handoff status string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseHandoffStatusError(String);

impl std::fmt::Display for ParseHandoffStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid handoff status: '{}'", self.0)
    }
}

impl std::error::Error for ParseHandoffStatusError {}

impl TryFrom<&str> for HandoffStatus {
    type Error = ParseHandoffStatusError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "initiated" => Ok(Self::Initiated),
            "accepted" => Ok(Self::Accepted),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(ParseHandoffStatusError(s.to_owned())),
        }
    }
}
