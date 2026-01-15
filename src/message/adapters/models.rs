//! Diesel model types for message persistence.
//!
//! These types map database rows to Rust structs using Diesel's derive macros.
//! They serve as the boundary between the database and domain layers.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use super::schema::{conversations, domain_events, messages};

// ============================================================================
// Conversation Models
// ============================================================================

/// Database row representation of a conversation.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = conversations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ConversationRow {
    /// Unique conversation identifier.
    pub id: Uuid,
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
    pub fn new(id: Uuid, now: DateTime<Utc>) -> Self {
        Self {
            id,
            task_id: None,
            context: Value::Object(serde_json::Map::new()),
            state: "active".to_owned(),
            created_at: now,
            updated_at: now,
        }
    }
}

// ============================================================================
// Message Models
// ============================================================================

/// Database row representation of a message.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MessageRow {
    /// Unique message identifier.
    pub id: Uuid,
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

// ============================================================================
// Domain Event Models
// ============================================================================

/// Database row representation of a domain event.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = domain_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DomainEventRow {
    /// Unique event identifier.
    pub id: Uuid,
    /// The aggregate this event applies to.
    pub aggregate_id: Uuid,
    /// Type of aggregate.
    pub aggregate_type: String,
    /// Type of event.
    pub event_type: String,
    /// Event payload.
    pub event_data: Value,
    /// Schema version.
    pub event_version: i32,
    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,
    /// Correlation ID for tracing.
    pub correlation_id: Option<Uuid>,
    /// Causation ID.
    pub causation_id: Option<Uuid>,
    /// User who caused the event.
    pub user_id: Option<Uuid>,
    /// Session context.
    pub session_id: Option<Uuid>,
}

/// Data for inserting a new domain event.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = domain_events)]
pub struct NewDomainEvent {
    /// Unique event identifier.
    pub id: Uuid,
    /// The aggregate this event applies to.
    pub aggregate_id: Uuid,
    /// Type of aggregate.
    pub aggregate_type: String,
    /// Type of event.
    pub event_type: String,
    /// Event payload.
    pub event_data: Value,
    /// Schema version.
    pub event_version: i32,
    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,
    /// Correlation ID for tracing.
    pub correlation_id: Option<Uuid>,
    /// Causation ID.
    pub causation_id: Option<Uuid>,
    /// User who caused the event.
    pub user_id: Option<Uuid>,
    /// Session context.
    pub session_id: Option<Uuid>,
}
