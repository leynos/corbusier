//! Domain identifier newtypes providing type safety for message, conversation, and
//! session identifiers.
//!
//! These types wrap UUIDs to prevent accidental mixing of different identifier types
//! and to provide domain-specific validation.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a message within the Corbusier system.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::MessageId;
///
/// let id = MessageId::new();
/// assert!(!id.as_ref().is_nil());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(Uuid);

impl MessageId {
    /// Creates a new random message identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a message identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID value.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

/// Note: This implementation generates a new random UUID on each call,
/// which is non-standard behaviour for `Default`. Use `MessageId::new()`
/// if the intent to generate a random ID should be explicit.
impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for MessageId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a conversation thread.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::ConversationId;
///
/// let id = ConversationId::new();
/// assert!(!id.as_ref().is_nil());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConversationId(Uuid);

impl ConversationId {
    /// Creates a new random conversation identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a conversation identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID value.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for ConversationId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for ConversationId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for ConversationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Turn identifier for tracking conversation turns.
///
/// A turn represents a single interaction cycle between the user and an agent,
/// potentially including multiple tool calls.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::TurnId;
///
/// let id = TurnId::new();
/// assert!(!id.as_ref().is_nil());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TurnId(Uuid);

impl TurnId {
    /// Creates a new random turn identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a turn identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID value.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for TurnId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for TurnId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for TurnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Sequence number for ordering messages within a conversation.
///
/// Sequence numbers are monotonically increasing within a conversation,
/// ensuring deterministic message ordering.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::SequenceNumber;
///
/// let seq = SequenceNumber::new(1);
/// assert_eq!(seq.value(), 1);
/// assert_eq!(seq.next().value(), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SequenceNumber(u64);

impl SequenceNumber {
    /// Creates a sequence number from a value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying sequence value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.0
    }

    /// Returns the next sequence number.
    ///
    /// Uses saturating arithmetic, so at `u64::MAX` it will not overflow
    /// but return `u64::MAX`. This is practically unreachable in normal use
    /// (would require 2^64 messages).
    #[must_use]
    pub const fn next(&self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl From<u64> for SequenceNumber {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl fmt::Display for SequenceNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an agent handoff event.
///
/// A handoff represents a transfer of conversation control from one agent
/// backend to another, preserving context and audit trail.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::HandoffId;
///
/// let id = HandoffId::new();
/// assert!(!id.as_ref().is_nil());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HandoffId(Uuid);

impl HandoffId {
    /// Creates a new random handoff identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a handoff identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID value.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for HandoffId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for HandoffId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for HandoffId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for an agent session within a conversation.
///
/// An agent session represents a contiguous period where a single agent backend
/// handles turns within a conversation. Sessions are created when an agent
/// begins processing and end via handoff or completion.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::AgentSessionId;
///
/// let id = AgentSessionId::new();
/// assert!(!id.as_ref().is_nil());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentSessionId(Uuid);

impl AgentSessionId {
    /// Creates a new random agent session identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an agent session identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID value.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for AgentSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for AgentSessionId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for AgentSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
