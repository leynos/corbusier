//! In-memory implementation of the `MessageRepository` port.
//!
//! Provides a simple, thread-safe repository for unit testing
//! without database dependencies. Not suitable for production use.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::message::{
    domain::{ConversationId, Message, MessageId, SequenceNumber},
    error::RepositoryError,
    ports::repository::{MessageRepository, RepositoryResult},
};

/// Error indicating a duplicate message ID was detected.
///
/// Used by the in-memory adapter to report uniqueness violations
/// in a backend-agnostic way via [`RepositoryError::database`].
#[derive(Debug)]
struct DuplicateIdError {
    id: MessageId,
}

impl fmt::Display for DuplicateIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "message with id {} already exists", self.id)
    }
}

impl std::error::Error for DuplicateIdError {}

/// In-memory implementation of [`MessageRepository`].
///
/// Thread-safe via internal [`RwLock`]. Suitable for unit tests only.
///
/// # Example
///
/// ```
/// use corbusier::message::adapters::memory::InMemoryMessageRepository;
/// use corbusier::message::ports::repository::MessageRepository;
///
/// let repo = InMemoryMessageRepository::new();
/// // Use repo in tests...
/// ```
#[derive(Debug, Default, Clone)]
pub struct InMemoryMessageRepository {
    messages: Arc<RwLock<HashMap<MessageId, Message>>>,
}

impl InMemoryMessageRepository {
    /// Creates an empty repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of stored messages.
    ///
    /// Returns `0` if the internal lock is poisoned, matching the fallback
    /// behaviour of an empty repository. For error-propagating access, use
    /// the repository trait methods instead.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.read().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Returns `true` if no messages are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl MessageRepository for InMemoryMessageRepository {
    async fn store(&self, message: &Message) -> RepositoryResult<()> {
        let mut guard = self
            .messages
            .write()
            .map_err(|e| RepositoryError::connection(format!("lock poisoned: {e}")))?;

        if guard.contains_key(&message.id()) {
            return Err(RepositoryError::database(DuplicateIdError {
                id: message.id(),
            }));
        }

        guard.insert(message.id(), message.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: MessageId) -> RepositoryResult<Option<Message>> {
        let guard = self
            .messages
            .read()
            .map_err(|e| RepositoryError::connection(format!("lock poisoned: {e}")))?;

        Ok(guard.get(&id).cloned())
    }

    async fn find_by_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<Vec<Message>> {
        let guard = self
            .messages
            .read()
            .map_err(|e| RepositoryError::connection(format!("lock poisoned: {e}")))?;

        let mut messages: Vec<Message> = guard
            .values()
            .filter(|m| m.conversation_id() == conversation_id)
            .cloned()
            .collect();

        // Sort by sequence number for consistent ordering
        messages.sort_by_key(|m| m.sequence_number().value());

        Ok(messages)
    }

    async fn next_sequence_number(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<SequenceNumber> {
        let guard = self
            .messages
            .read()
            .map_err(|e| RepositoryError::connection(format!("lock poisoned: {e}")))?;

        let max_seq = guard
            .values()
            .filter(|m| m.conversation_id() == conversation_id)
            .map(|m| m.sequence_number().value())
            .max()
            .unwrap_or(0);

        Ok(SequenceNumber::new(max_seq.saturating_add(1)))
    }

    async fn exists(&self, id: MessageId) -> RepositoryResult<bool> {
        let guard = self
            .messages
            .read()
            .map_err(|e| RepositoryError::connection(format!("lock poisoned: {e}")))?;

        Ok(guard.contains_key(&id))
    }
}
