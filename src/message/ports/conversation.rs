//! Repository port for conversation persistence.

use crate::context::RequestContext;
use crate::message::domain::{Conversation, ConversationId};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for conversation repository operations.
pub type ConversationRepositoryResult<T> = Result<T, ConversationRepositoryError>;

/// Persistence contract for conversations.
#[async_trait]
pub trait ConversationRepository: Send + Sync {
    /// Stores a new conversation.
    ///
    /// # Errors
    ///
    /// Returns [`ConversationRepositoryError::DuplicateConversation`] when the
    /// identifier already exists.
    async fn store(
        &self,
        ctx: &RequestContext,
        conversation: &Conversation,
    ) -> ConversationRepositoryResult<()>;

    /// Finds a conversation by identifier.
    ///
    /// Returns `None` when the conversation does not exist.
    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> ConversationRepositoryResult<Option<Conversation>>;
}

/// Errors returned by conversation repositories.
#[derive(Debug, Clone, Error)]
pub enum ConversationRepositoryError {
    /// Conversation identifier already exists.
    #[error("duplicate conversation identifier: {0}")]
    DuplicateConversation(ConversationId),

    /// Persistence-layer failure.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl ConversationRepositoryError {
    /// Wraps a persistence error.
    #[must_use]
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
