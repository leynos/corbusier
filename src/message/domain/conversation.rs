//! Conversation aggregate root for message-history workflows.
//!
//! Conversations group immutable messages into a single thread and provide the
//! anchor entity used by the HTTP conversation API.

use super::ConversationId;
use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};

/// Conversation lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationState {
    /// Conversation is active and accepts new messages.
    Active,
    /// Conversation is paused.
    Paused,
    /// Conversation is archived.
    Archived,
}

impl ConversationState {
    /// Returns the canonical storage representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Archived => "archived",
        }
    }
}

/// Error type for invalid conversation state strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationStateParseError(pub String);

impl std::fmt::Display for ConversationStateParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown conversation state: {}", self.0)
    }
}

impl std::error::Error for ConversationStateParseError {}

impl TryFrom<&str> for ConversationState {
    type Error = ConversationStateParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "archived" => Ok(Self::Archived),
            _ => Err(ConversationStateParseError(value.to_owned())),
        }
    }
}

/// Conversation aggregate root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    id: ConversationId,
    state: ConversationState,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Conversation {
    /// Creates a new active conversation.
    #[must_use]
    pub fn new(clock: &impl Clock) -> Self {
        let now = clock.utc();
        Self {
            id: ConversationId::new(),
            state: ConversationState::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstructs a persisted conversation.
    #[must_use]
    pub const fn from_persisted(
        id: ConversationId,
        state: ConversationState,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            state,
            created_at,
            updated_at,
        }
    }

    /// Returns the conversation identifier.
    #[must_use]
    pub const fn id(&self) -> ConversationId {
        self.id
    }

    /// Returns the conversation state.
    #[must_use]
    pub const fn state(&self) -> ConversationState {
        self.state
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns the latest update timestamp.
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}
