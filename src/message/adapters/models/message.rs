//! Diesel models for message persistence.
//!
//! Maps database rows to Rust structs for the `messages` table.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use super::super::schema::messages;

/// Database row representation of a message.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MessageRow {
    /// Unique message identifier.
    pub id: Uuid,
    /// Owning tenant identifier.
    pub tenant_id: Uuid,
    /// Reference to the containing conversation.
    pub conversation_id: Uuid,
    /// Message role: user, assistant, tool, or system.
    pub role: String,
    /// Message content parts as JSONB.
    pub content: Value,
    /// Message metadata as JSONB.
    pub metadata: Value,
    /// When the message was created.
    pub created_at: DateTime<Utc>,
    /// Sequence number for ordering.
    pub sequence_number: i64,
}

/// Data for inserting a new message.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = messages)]
pub struct NewMessage {
    /// Unique message identifier.
    pub id: Uuid,
    /// Owning tenant identifier.
    pub tenant_id: Uuid,
    /// Reference to the containing conversation.
    pub conversation_id: Uuid,
    /// Message role: user, assistant, tool, or system.
    pub role: String,
    /// Message content parts as JSONB.
    pub content: Value,
    /// Message metadata as JSONB.
    pub metadata: Value,
    /// When the message was created.
    pub created_at: DateTime<Utc>,
    /// Sequence number for ordering.
    pub sequence_number: i64,
}

impl NewMessage {
    /// Creates a `NewMessage` from a domain `Message`.
    ///
    /// Serializes the message content and metadata to JSONB and converts
    /// the sequence number to `i64`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::message::error::RepositoryError::Serialization`] if:
    /// - Content or metadata cannot be serialized to JSON
    /// - Sequence number overflows `i64`
    pub fn try_from_domain(
        message: &crate::message::domain::Message,
        tenant_id: Uuid,
    ) -> crate::message::ports::repository::RepositoryResult<Self> {
        use crate::message::error::RepositoryError;

        let content = serde_json::to_value(message.content())
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        let metadata = serde_json::to_value(message.metadata())
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        let sequence_number = i64::try_from(message.sequence_number().value())
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        Ok(Self {
            id: message.id().into_inner(),
            tenant_id,
            conversation_id: message.conversation_id().into_inner(),
            role: message.role().as_str().to_owned(),
            content,
            metadata,
            created_at: message.created_at(),
            sequence_number,
        })
    }
}
