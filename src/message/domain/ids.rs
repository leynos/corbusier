//! Domain identifier newtypes providing type safety for message and conversation IDs.
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
    /// # Panics
    ///
    /// This method will not panic under normal use as u64 overflow is
    /// practically unreachable (would require 2^64 messages).
    #[must_use]
    pub const fn next(&self) -> Self {
        Self(self.0 + 1)
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
