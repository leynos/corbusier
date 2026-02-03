//! Context window snapshot types for capturing agent session state.
//!
//! Snapshots record the state of the context window at key points in an agent
//! session's lifecycle, enabling complete reconstruction of what was visible
//! to the agent at any given moment.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::handoff::ToolCallReference;
use super::{AgentSessionId, ConversationId, SequenceNumber};

/// A snapshot of the context window at a point in time.
///
/// Captures the state visible to an agent at the start or end of a session,
/// enabling complete context reconstruction for handoffs and auditing.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::{
///     AgentSessionId, ConversationId, ContextWindowSnapshot, MessageSummary,
///     SequenceRange, SequenceNumber, SnapshotParams, SnapshotType,
/// };
/// use mockable::DefaultClock;
///
/// let clock = DefaultClock;
/// let params = SnapshotParams::new(
///     ConversationId::new(),
///     AgentSessionId::new(),
///     SequenceRange::new(SequenceNumber::new(1), SequenceNumber::new(10)),
///     MessageSummary::new(5, 4, 1, 0),
///     SnapshotType::SessionStart,
/// );
/// let snapshot = ContextWindowSnapshot::new(params, &clock);
/// assert_eq!(snapshot.message_summary.total(), 10);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextWindowSnapshot {
    /// Unique identifier for this snapshot.
    pub snapshot_id: Uuid,

    /// The conversation this snapshot belongs to.
    pub conversation_id: ConversationId,

    /// The agent session this snapshot represents.
    pub session_id: AgentSessionId,

    /// The sequence number range included in this context window.
    pub sequence_range: SequenceRange,

    /// Summary of messages included (count by role).
    pub message_summary: MessageSummary,

    /// Tool calls visible in this context window.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visible_tool_calls: Vec<ToolCallReference>,

    /// Token count estimate for this context window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_estimate: Option<u64>,

    /// When the snapshot was captured.
    pub captured_at: DateTime<Utc>,

    /// Type of snapshot.
    pub snapshot_type: SnapshotType,
}

/// Parameters for creating a context window snapshot.
#[derive(Debug, Clone, Copy)]
pub struct SnapshotParams {
    /// The conversation this snapshot belongs to.
    pub conversation_id: ConversationId,
    /// The agent session this snapshot represents.
    pub session_id: AgentSessionId,
    /// The sequence number range included in this context window.
    pub sequence_range: SequenceRange,
    /// Summary of messages included (count by role).
    pub message_summary: MessageSummary,
    /// Type of snapshot.
    pub snapshot_type: SnapshotType,
}

impl SnapshotParams {
    /// Creates new snapshot parameters.
    ///
    /// Note: This constructor intentionally has 5 arguments as it serves as a
    /// parameter holder pattern to reduce argument counts in other functions.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "parameter struct constructor holds required fields"
    )]
    pub const fn new(
        conversation_id: ConversationId,
        session_id: AgentSessionId,
        sequence_range: SequenceRange,
        message_summary: MessageSummary,
        snapshot_type: SnapshotType,
    ) -> Self {
        Self {
            conversation_id,
            session_id,
            sequence_range,
            message_summary,
            snapshot_type,
        }
    }
}

impl ContextWindowSnapshot {
    /// Creates a new context window snapshot.
    #[must_use]
    pub fn new(params: SnapshotParams, clock: &impl mockable::Clock) -> Self {
        Self {
            snapshot_id: Uuid::new_v4(),
            conversation_id: params.conversation_id,
            session_id: params.session_id,
            sequence_range: params.sequence_range,
            message_summary: params.message_summary,
            visible_tool_calls: Vec::new(),
            token_estimate: None,
            captured_at: clock.utc(),
            snapshot_type: params.snapshot_type,
        }
    }

    /// Adds a visible tool call to the snapshot.
    #[must_use]
    pub fn with_visible_tool_call(mut self, reference: ToolCallReference) -> Self {
        self.visible_tool_calls.push(reference);
        self
    }

    /// Adds multiple visible tool calls.
    #[must_use]
    pub fn with_visible_tool_calls(
        mut self,
        references: impl IntoIterator<Item = ToolCallReference>,
    ) -> Self {
        self.visible_tool_calls.extend(references);
        self
    }

    /// Sets the token estimate.
    #[must_use]
    pub const fn with_token_estimate(mut self, estimate: u64) -> Self {
        self.token_estimate = Some(estimate);
        self
    }
}

/// The sequence number range for a context window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceRange {
    /// First sequence number in the range (inclusive).
    pub start: SequenceNumber,

    /// Last sequence number in the range (inclusive).
    pub end: SequenceNumber,
}

impl SequenceRange {
    /// Creates a new sequence range.
    #[must_use]
    pub const fn new(start: SequenceNumber, end: SequenceNumber) -> Self {
        Self { start, end }
    }

    /// Returns the number of messages in this range.
    #[must_use]
    pub const fn len(&self) -> u64 {
        self.end.value().saturating_sub(self.start.value()) + 1
    }

    /// Returns `true` if the range is empty (end < start).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.end.value() < self.start.value()
    }

    /// Returns `true` if the given sequence number is within this range.
    #[must_use]
    pub fn contains(&self, seq: SequenceNumber) -> bool {
        seq >= self.start && seq <= self.end
    }
}

/// Summary of messages in a context window, grouped by role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MessageSummary {
    /// Number of user messages.
    pub user_count: u32,

    /// Number of assistant messages.
    pub assistant_count: u32,

    /// Number of tool messages.
    pub tool_count: u32,

    /// Number of system messages.
    pub system_count: u32,
}

impl MessageSummary {
    /// Creates a new message summary.
    #[must_use]
    pub const fn new(user: u32, assistant: u32, tool: u32, system: u32) -> Self {
        Self {
            user_count: user,
            assistant_count: assistant,
            tool_count: tool,
            system_count: system,
        }
    }

    /// Returns the total number of messages.
    #[must_use]
    pub const fn total(&self) -> u32 {
        self.user_count + self.assistant_count + self.tool_count + self.system_count
    }

    /// Returns `true` if there are no messages.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

/// Type of context snapshot.
///
/// Distinguishes between different capture contexts for debugging and analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotType {
    /// Captured at the start of an agent session.
    SessionStart,

    /// Captured at handoff initiation.
    HandoffInitiated,

    /// Captured when context window is truncated.
    Truncation,

    /// Periodic checkpoint.
    Checkpoint,
}

impl SnapshotType {
    /// Returns the snapshot type as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::HandoffInitiated => "handoff_initiated",
            Self::Truncation => "truncation",
            Self::Checkpoint => "checkpoint",
        }
    }
}

impl std::fmt::Display for SnapshotType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an invalid snapshot type string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSnapshotTypeError(String);

impl std::fmt::Display for ParseSnapshotTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid snapshot type: '{}'", self.0)
    }
}

impl std::error::Error for ParseSnapshotTypeError {}

impl TryFrom<&str> for SnapshotType {
    type Error = ParseSnapshotTypeError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "session_start" => Ok(Self::SessionStart),
            "handoff_initiated" => Ok(Self::HandoffInitiated),
            "truncation" => Ok(Self::Truncation),
            "checkpoint" => Ok(Self::Checkpoint),
            _ => Err(ParseSnapshotTypeError(s.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockable::DefaultClock;

    #[test]
    fn context_snapshot_creation() {
        let clock = DefaultClock;
        let conv_id = ConversationId::new();
        let session_id = AgentSessionId::new();
        let range = SequenceRange::new(SequenceNumber::new(1), SequenceNumber::new(10));
        let summary = MessageSummary::new(5, 4, 1, 0);

        let params = SnapshotParams::new(
            conv_id,
            session_id,
            range,
            summary,
            SnapshotType::SessionStart,
        );
        let snapshot = ContextWindowSnapshot::new(params, &clock);

        assert_eq!(snapshot.conversation_id, conv_id);
        assert_eq!(snapshot.session_id, session_id);
        assert_eq!(snapshot.sequence_range, range);
        assert_eq!(snapshot.message_summary.total(), 10);
        assert!(snapshot.visible_tool_calls.is_empty());
        assert!(snapshot.token_estimate.is_none());
    }

    #[test]
    fn sequence_range_len() {
        let range = SequenceRange::new(SequenceNumber::new(5), SequenceNumber::new(10));
        assert_eq!(range.len(), 6);
        assert!(!range.is_empty());

        let single = SequenceRange::new(SequenceNumber::new(1), SequenceNumber::new(1));
        assert_eq!(single.len(), 1);
    }

    #[test]
    fn sequence_range_contains() {
        let range = SequenceRange::new(SequenceNumber::new(5), SequenceNumber::new(10));

        assert!(!range.contains(SequenceNumber::new(4)));
        assert!(range.contains(SequenceNumber::new(5)));
        assert!(range.contains(SequenceNumber::new(7)));
        assert!(range.contains(SequenceNumber::new(10)));
        assert!(!range.contains(SequenceNumber::new(11)));
    }

    #[test]
    fn message_summary_total() {
        let summary = MessageSummary::new(3, 2, 1, 0);
        assert_eq!(summary.total(), 6);
        assert!(!summary.is_empty());

        let empty = MessageSummary::default();
        assert_eq!(empty.total(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn snapshot_type_serialisation() {
        assert_eq!(
            serde_json::to_string(&SnapshotType::SessionStart).expect("serialisation"),
            "\"session_start\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotType::HandoffInitiated).expect("serialisation"),
            "\"handoff_initiated\""
        );
    }
}
