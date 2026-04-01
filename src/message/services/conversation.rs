//! Conversation workflow service.

use crate::context::RequestContext;
use crate::message::{
    domain::{ContentPart, Conversation, ConversationId, Message, MessageBuilderError, Role},
    error::{RepositoryError, ValidationError},
    ports::{
        MessageRepository, MessageValidator,
        conversation::{ConversationRepository, ConversationRepositoryError},
    },
};
use mockable::Clock;
use std::sync::Arc;
use thiserror::Error;

/// Request payload for appending a message to a conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct AppendMessageRequest {
    conversation_id: ConversationId,
    role: Role,
    content: Vec<ContentPart>,
}

impl AppendMessageRequest {
    /// Creates a request with required fields.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Kept non-const per design decision to allow future non-const initialisation"
    )]
    pub fn new(conversation_id: ConversationId, role: Role, content: Vec<ContentPart>) -> Self {
        Self {
            conversation_id,
            role,
            content,
        }
    }
}

/// Service-level errors for conversation workflows.
#[derive(Debug, Error)]
pub enum ConversationServiceError {
    /// Conversation does not exist.
    #[error("conversation not found: {0}")]
    ConversationNotFound(ConversationId),
    /// Conversation repository failure.
    #[error(transparent)]
    ConversationRepository(#[from] ConversationRepositoryError),
    /// Message repository failure.
    #[error(transparent)]
    MessageRepository(#[from] RepositoryError),
    /// Message validation failure.
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

/// Result type for conversation service operations.
pub type ConversationServiceResult<T> = Result<T, ConversationServiceError>;

/// Conversation application service.
#[derive(Clone)]
pub struct ConversationService<ConvoRepo, MessageRepo, Validator, C>
where
    ConvoRepo: ConversationRepository,
    MessageRepo: MessageRepository,
    Validator: MessageValidator,
    C: Clock + Send + Sync,
{
    conversation_repository: Arc<ConvoRepo>,
    message_repository: Arc<MessageRepo>,
    validator: Arc<Validator>,
    clock: Arc<C>,
}

impl<ConvoRepo, MessageRepo, Validator, C> ConversationService<ConvoRepo, MessageRepo, Validator, C>
where
    ConvoRepo: ConversationRepository,
    MessageRepo: MessageRepository,
    Validator: MessageValidator,
    C: Clock + Send + Sync,
{
    /// Creates a new conversation service.
    #[must_use]
    pub const fn new(
        conversation_repository: Arc<ConvoRepo>,
        message_repository: Arc<MessageRepo>,
        validator: Arc<Validator>,
        clock: Arc<C>,
    ) -> Self {
        Self {
            conversation_repository,
            message_repository,
            validator,
            clock,
        }
    }

    /// Creates a new empty conversation.
    ///
    /// # Errors
    ///
    /// Returns repository errors when persistence fails.
    pub async fn create_conversation(
        &self,
        ctx: &RequestContext,
    ) -> ConversationServiceResult<Conversation> {
        let conversation = Conversation::new(&*self.clock);
        self.conversation_repository
            .store(ctx, &conversation)
            .await?;
        Ok(conversation)
    }

    /// Returns conversation history ordered by sequence number.
    ///
    /// # Errors
    ///
    /// Returns [`ConversationServiceError::ConversationNotFound`] when the
    /// conversation does not exist.
    pub async fn history(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> ConversationServiceResult<Vec<Message>> {
        self.require_conversation(ctx, conversation_id).await?;
        self.message_repository
            .find_by_conversation(ctx, conversation_id)
            .await
            .map_err(Into::into)
    }

    /// Appends a message to an existing conversation.
    ///
    /// # Errors
    ///
    /// Returns [`ConversationServiceError::ConversationNotFound`] when the
    /// conversation does not exist, or validation/repository errors when
    /// message construction fails.
    pub async fn append_message(
        &self,
        ctx: &RequestContext,
        request: AppendMessageRequest,
    ) -> ConversationServiceResult<Message> {
        let AppendMessageRequest {
            conversation_id,
            role,
            content,
        } = request;
        self.require_conversation(ctx, conversation_id).await?;
        let next_sequence = self
            .message_repository
            .next_sequence_number(ctx, conversation_id)
            .await?;
        let message = Message::builder(conversation_id, role, next_sequence)
            .with_content_parts(content)
            .build(&*self.clock)
            .map_err(|error| Self::builder_error_to_validation(&error))?;
        self.validator.validate(&message)?;
        self.message_repository.store(ctx, &message).await?;
        Ok(message)
    }

    async fn require_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> ConversationServiceResult<Conversation> {
        self.conversation_repository
            .find_by_id(ctx, conversation_id)
            .await?
            .ok_or(ConversationServiceError::ConversationNotFound(
                conversation_id,
            ))
    }

    const fn builder_error_to_validation(error: &MessageBuilderError) -> ValidationError {
        match error {
            MessageBuilderError::EmptyContent => ValidationError::EmptyContent,
        }
    }
}
