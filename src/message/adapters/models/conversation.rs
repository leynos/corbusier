//! Diesel models for conversation persistence.
//!
//! Maps database rows to Rust structs for the `conversations` table.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use super::super::schema::conversations;
use crate::message::domain::ConversationState;

/// Database row representation of a conversation.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = conversations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ConversationRow {
    /// Unique conversation identifier.
    pub id: Uuid,
    /// Tenant that owns this conversation.
    pub tenant_id: Uuid,
    /// Optional reference to the associated task.
    pub task_id: Option<Uuid>,
    /// Flexible context data.
    pub context: Value,
    /// Conversation state.
    pub state: String,
    /// When the conversation was created.
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Data for inserting a new conversation.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = conversations)]
pub struct NewConversation {
    /// Unique conversation identifier.
    pub id: Uuid,
    /// Tenant that owns this conversation.
    pub tenant_id: Uuid,
    /// Optional reference to the associated task.
    pub task_id: Option<Uuid>,
    /// Flexible context data.
    pub context: Value,
    /// Conversation state.
    pub state: String,
    /// When the conversation was created.
    pub created_at: DateTime<Utc>,
    /// When the conversation was last updated.
    pub updated_at: DateTime<Utc>,
}

impl NewConversation {
    /// Creates a new conversation record with default state.
    #[must_use]
    pub fn new(id: Uuid, tenant_id: Uuid, now: DateTime<Utc>) -> Self {
        Self {
            id,
            tenant_id,
            task_id: None,
            context: Value::Object(serde_json::Map::new()),
            state: ConversationState::Active.as_str().to_owned(),
            created_at: now,
            updated_at: now,
        }
    }
}
