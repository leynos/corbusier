//! Message metadata types capturing contextual information about messages.

use super::TurnId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Metadata associated with a message.
///
/// Captures information about the message's origin, processing context,
/// and any extension data required by specific workflows.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::{MessageMetadata, TurnId};
///
/// let metadata = MessageMetadata::with_agent_backend("claude_code_sdk")
///     .with_turn_id(TurnId::new());
/// assert_eq!(metadata.agent_backend, Some("claude_code_sdk".to_string()));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// The agent backend that produced this message (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_backend: Option<String>,

    /// The turn identifier within which this message was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,

    /// Slash command expansion details (if this message resulted from a command).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slash_command_expansion: Option<SlashCommandExpansion>,

    /// Extension data for custom metadata fields.
    ///
    /// **Warning:** Due to `#[serde(flatten)]`, any JSON keys not matching known
    /// fields during deserialisation will be captured here. This can cause
    /// unexpected behaviour if an extension key collides with a future field name.
    /// Avoid using keys like `agent_backend`, `turn_id`, or `slash_command_expansion`.
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, Value>,
}

impl MessageMetadata {
    /// Creates empty metadata.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates metadata with an agent backend specified.
    #[must_use]
    pub fn with_agent_backend(agent_backend: impl Into<String>) -> Self {
        Self {
            agent_backend: Some(agent_backend.into()),
            ..Default::default()
        }
    }

    /// Sets the turn identifier.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::Some with Copy type should be const but isn't stable"
    )]
    pub fn with_turn_id(mut self, turn_id: TurnId) -> Self {
        self.turn_id = Some(turn_id);
        self
    }

    /// Sets the slash command expansion details.
    #[must_use]
    pub fn with_slash_command_expansion(mut self, expansion: SlashCommandExpansion) -> Self {
        self.slash_command_expansion = Some(expansion);
        self
    }

    /// Adds an extension field.
    #[must_use]
    pub fn with_extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }

    /// Returns `true` if the metadata is empty (no fields set).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.agent_backend.is_none()
            && self.turn_id.is_none()
            && self.slash_command_expansion.is_none()
            && self.extensions.is_empty()
    }
}

/// Details about a slash command expansion that produced a message.
///
/// When a user invokes a slash command (e.g., `/review`), the command is
/// expanded into a template that generates one or more messages. This
/// structure records the expansion details for audit and debugging.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlashCommandExpansion {
    /// The original command string (e.g., "/review").
    pub command: String,
    /// Parameters passed to the command.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub parameters: HashMap<String, Value>,
    /// The expanded template result.
    pub expanded_content: String,
}

impl SlashCommandExpansion {
    /// Creates a new slash command expansion record.
    #[must_use]
    pub fn new(command: impl Into<String>, expanded_content: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            parameters: HashMap::new(),
            expanded_content: expanded_content.into(),
        }
    }

    /// Adds a parameter to the expansion.
    #[must_use]
    pub fn with_parameter(mut self, key: impl Into<String>, value: Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }
}
