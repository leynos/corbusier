//! Conversion helpers for `PostgreSQL` repository.
//!
//! Provides functions for converting between database rows and domain types.

use super::super::models::MessageRow;
use crate::message::{
    domain::{
        ContentPart, ConversationId, Message, MessageId, MessageMetadata, Role, SequenceNumber,
    },
    error::RepositoryError,
    ports::repository::RepositoryResult,
};

/// Wraps a serialization/conversion error for consistent error handling.
pub(super) fn ser_err<E: std::fmt::Display>(e: E) -> RepositoryError {
    RepositoryError::serialization(e.to_string())
}

/// Converts a database row to a domain Message.
///
/// This function deserializes the role, content, and metadata from their
/// stored representations and reconstructs the domain Message using
/// [`Message::from_persisted`].
///
/// # Errors
///
/// Returns [`RepositoryError::Serialization`] if:
/// - The role string is not a valid [`Role`] variant
/// - The content JSONB cannot be deserialized to `Vec<ContentPart>`
/// - The metadata JSONB cannot be deserialized to [`MessageMetadata`]
/// - The sequence number is negative (invalid for `u64`)
/// - The content is empty (domain invariant violation)
pub(crate) fn row_to_message(row: MessageRow) -> RepositoryResult<Message> {
    let role = Role::try_from(row.role.as_str()).map_err(ser_err)?;
    let content: Vec<ContentPart> = serde_json::from_value(row.content).map_err(ser_err)?;
    let metadata: MessageMetadata = serde_json::from_value(row.metadata).map_err(ser_err)?;
    let sequence_number = u64::try_from(row.sequence_number).map_err(ser_err)?;

    Message::from_persisted(
        MessageId::from_uuid(row.id),
        ConversationId::from_uuid(row.conversation_id),
        role,
        content,
        metadata,
        row.created_at,
        SequenceNumber::new(sequence_number),
    )
    .map_err(ser_err)
}
