//! The Message aggregate root representing a single message in a conversation.
//!
//! Messages are immutable after creation and contain all information needed
//! to reconstruct the conversation state.

use super::{ContentPart, ConversationId, MessageId, MessageMetadata, Role, SequenceNumber};
use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};

/// A message within a conversation.
///
/// Messages are the atomic unit of conversation history in Corbusier.
/// They are immutable after creation and carry all necessary context
/// for audit trails and conversation reconstruction.
///
/// # Invariants
///
/// - `id` is always a valid, non-nil UUID
/// - `created_at` is always populated
/// - `content` contains at least one part (enforced at construction)
/// - Messages cannot be modified after creation
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::{
///     ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart,
/// };
/// use mockable::DefaultClock;
///
/// let clock = DefaultClock;
/// let message = Message::new(
///     ConversationId::new(),
///     Role::User,
///     vec![ContentPart::Text(TextPart::new("Hello!"))],
///     SequenceNumber::new(1),
///     &clock,
/// ).expect("valid message");
///
/// assert_eq!(message.role(), Role::User);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message.
    id: MessageId,

    /// The conversation this message belongs to.
    conversation_id: ConversationId,

    /// The role of the message source.
    role: Role,

    /// The content parts of this message.
    content: Vec<ContentPart>,

    /// Associated metadata.
    metadata: MessageMetadata,

    /// When the message was created.
    created_at: DateTime<Utc>,

    /// The sequence number within the conversation.
    sequence_number: SequenceNumber,
}

impl Message {
    /// Creates a new message with the current timestamp.
    ///
    /// # Errors
    ///
    /// Returns [`MessageBuilderError::EmptyContent`] if the content is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use corbusier::message::domain::{
    ///     ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart,
    /// };
    /// use mockable::DefaultClock;
    ///
    /// let clock = DefaultClock;
    /// let result = Message::new(
    ///     ConversationId::new(),
    ///     Role::User,
    ///     vec![ContentPart::Text(TextPart::new("Hello"))],
    ///     SequenceNumber::new(1),
    ///     &clock,
    /// );
    /// assert!(result.is_ok());
    /// ```
    #[expect(
        clippy::too_many_arguments,
        reason = "Factory function requires all fields for direct construction"
    )]
    pub fn new(
        conversation_id: ConversationId,
        role: Role,
        content: Vec<ContentPart>,
        sequence_number: SequenceNumber,
        clock: &impl Clock,
    ) -> Result<Self, MessageBuilderError> {
        Self::new_with_id(
            MessageId::new(),
            conversation_id,
            role,
            content,
            sequence_number,
            clock,
        )
    }

    /// Creates a new message with a specified ID.
    ///
    /// # Errors
    ///
    /// Returns [`MessageBuilderError::EmptyContent`] if the content is empty.
    #[expect(
        clippy::too_many_arguments,
        reason = "Factory function requires all fields for direct construction"
    )]
    pub fn new_with_id(
        id: MessageId,
        conversation_id: ConversationId,
        role: Role,
        content: Vec<ContentPart>,
        sequence_number: SequenceNumber,
        clock: &impl Clock,
    ) -> Result<Self, MessageBuilderError> {
        if content.is_empty() {
            return Err(MessageBuilderError::EmptyContent);
        }

        Ok(Self {
            id,
            conversation_id,
            role,
            content,
            metadata: MessageMetadata::empty(),
            created_at: clock.utc(),
            sequence_number,
        })
    }

    /// Returns the message identifier.
    #[must_use]
    pub const fn id(&self) -> MessageId {
        self.id
    }

    /// Returns the conversation identifier.
    #[must_use]
    pub const fn conversation_id(&self) -> ConversationId {
        self.conversation_id
    }

    /// Returns the message role.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }

    /// Returns the content parts.
    #[must_use]
    pub fn content(&self) -> &[ContentPart] {
        &self.content
    }

    /// Returns the metadata.
    #[must_use]
    pub const fn metadata(&self) -> &MessageMetadata {
        &self.metadata
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns the sequence number.
    #[must_use]
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    /// Returns a builder for constructing messages with metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use corbusier::message::domain::{
    ///     ContentPart, ConversationId, Message, MessageMetadata, Role,
    ///     SequenceNumber, TextPart,
    /// };
    /// use mockable::DefaultClock;
    ///
    /// let clock = DefaultClock;
    /// let message = Message::builder(ConversationId::new(), Role::Assistant, SequenceNumber::new(2))
    ///     .with_content(ContentPart::Text(TextPart::new("Response")))
    ///     .with_metadata(MessageMetadata::with_agent_backend("claude"))
    ///     .build(&clock)
    ///     .expect("valid message");
    /// ```
    #[must_use]
    pub fn builder(
        conversation_id: ConversationId,
        role: Role,
        sequence_number: SequenceNumber,
    ) -> MessageBuilder {
        MessageBuilder::new(conversation_id, role, sequence_number)
    }
}

/// Builder for constructing messages with full control over all fields.
#[derive(Debug)]
pub struct MessageBuilder {
    id: Option<MessageId>,
    conversation_id: ConversationId,
    role: Role,
    content: Vec<ContentPart>,
    metadata: MessageMetadata,
    sequence_number: SequenceNumber,
}

impl MessageBuilder {
    /// Creates a new message builder.
    #[must_use]
    pub fn new(
        conversation_id: ConversationId,
        role: Role,
        sequence_number: SequenceNumber,
    ) -> Self {
        Self {
            id: None,
            conversation_id,
            role,
            content: Vec::new(),
            metadata: MessageMetadata::empty(),
            sequence_number,
        }
    }

    /// Sets a specific message ID.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Option::Some with Copy type should be const but isn't stable"
    )]
    pub fn with_id(mut self, id: MessageId) -> Self {
        self.id = Some(id);
        self
    }

    /// Adds a content part.
    #[must_use]
    pub fn with_content(mut self, part: ContentPart) -> Self {
        self.content.push(part);
        self
    }

    /// Adds multiple content parts.
    #[must_use]
    pub fn with_content_parts(mut self, parts: impl IntoIterator<Item = ContentPart>) -> Self {
        self.content.extend(parts);
        self
    }

    /// Sets the metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: MessageMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Builds the message.
    ///
    /// # Errors
    ///
    /// Returns [`MessageBuilderError::EmptyContent`] if no content parts were added.
    pub fn build(self, clock: &impl Clock) -> Result<Message, MessageBuilderError> {
        if self.content.is_empty() {
            return Err(MessageBuilderError::EmptyContent);
        }

        let id = self.id.unwrap_or_default();

        Ok(Message {
            id,
            conversation_id: self.conversation_id,
            role: self.role,
            content: self.content,
            metadata: self.metadata,
            created_at: clock.utc(),
            sequence_number: self.sequence_number,
        })
    }
}

/// Errors that can occur when building a message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MessageBuilderError {
    /// The message content is empty.
    #[error("message must contain at least one content part")]
    EmptyContent,
}
