//! Conversation and message HTTP error mappings.

use super::ApiError;
use crate::message::error::RepositoryError;

pub(crate) fn map_conversation_repository_error(
    error: crate::message::ports::ConversationRepositoryError,
) -> ApiError {
    match error {
        crate::message::ports::ConversationRepositoryError::DuplicateConversation(id) => {
            ApiError::conflict("duplicate_conversation", id.to_string())
        }
        crate::message::ports::ConversationRepositoryError::Persistence(err) => {
            tracing::error!(error = %err, "conversation repository persistence error");
            ApiError::internal()
        }
    }
}

#[expect(
    clippy::cognitive_complexity,
    reason = "Simple match arms on error variants"
)]
pub(crate) fn map_message_repository_error(error: RepositoryError) -> ApiError {
    match error {
        RepositoryError::ConversationNotFound(conversation_id) => {
            ApiError::not_found("conversation_not_found", conversation_id.to_string())
        }
        RepositoryError::NotFound(message_id) => {
            ApiError::not_found("message_not_found", message_id.to_string())
        }
        RepositoryError::DuplicateMessage(message_id) => {
            ApiError::conflict("duplicate_message", message_id.to_string())
        }
        RepositoryError::DuplicateSequence {
            conversation_id,
            sequence,
        } => ApiError::conflict(
            "duplicate_sequence",
            format!("conversation {conversation_id} already has sequence {sequence}"),
        ),
        RepositoryError::Database(err) => {
            tracing::error!(error = %err, "message database error");
            ApiError::internal()
        }
        RepositoryError::Connection(err) => {
            tracing::error!(error = %err, "message connection error");
            ApiError::internal()
        }
        RepositoryError::Serialization(message) => {
            tracing::error!(error = %message, "message serialization error");
            ApiError::internal()
        }
    }
}
