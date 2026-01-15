//! Repository port for message persistence.
//!
//! Defines the abstract interface for storing and retrieving messages,
//! allowing different persistence implementations (`PostgreSQL`, in-memory, etc.).

use crate::message::{
    domain::{ConversationId, Message, MessageId, SequenceNumber},
    error::RepositoryError,
};
use async_trait::async_trait;

/// Result type for repository operations.
pub type RepositoryResult<T> = Result<T, RepositoryError>;

/// Port for message persistence operations.
///
/// Implementations provide the actual storage mechanism (`PostgreSQL`, `SQLite`,
/// in-memory for testing) while the domain logic remains storage-agnostic.
///
/// # Implementation Notes
///
/// Implementations must ensure:
/// - Message IDs are unique across the entire system
/// - Sequence numbers are unique within a conversation
/// - Messages are immutable after storage (no update operations)
/// - Concurrent access is handled safely
#[async_trait]
pub trait MessageRepository: Send + Sync {
    /// Stores a new message.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if:
    /// - A message with the same ID already exists
    /// - The database connection fails
    /// - Serialisation fails
    async fn store(&self, message: &Message) -> RepositoryResult<()>;

    /// Retrieves a message by its ID.
    ///
    /// Returns `None` if the message does not exist.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if the query fails.
    async fn find_by_id(&self, id: MessageId) -> RepositoryResult<Option<Message>>;

    /// Retrieves all messages for a conversation, ordered by sequence number.
    ///
    /// Returns an empty vector if no messages exist for the conversation.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if the query fails.
    async fn find_by_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<Vec<Message>>;

    /// Returns the next sequence number for a conversation.
    ///
    /// For a new conversation with no messages, returns `SequenceNumber::new(1)`.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if the query fails.
    async fn next_sequence_number(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<SequenceNumber>;

    /// Checks if a message with the given ID already exists.
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if the query fails.
    async fn exists(&self, id: MessageId) -> RepositoryResult<bool>;
}
