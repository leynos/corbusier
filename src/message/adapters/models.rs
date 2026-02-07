//! Diesel model types for message persistence.
//!
//! These types map database rows to Rust structs using Diesel's derive macros.
//! They serve as the boundary between the database and domain layers.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use super::schema::{
    agent_sessions, context_snapshots, conversations, domain_events, handoffs, messages,
};

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
            conversation_id: message.conversation_id().into_inner(),
            role: message.role().as_str().to_owned(),
            content,
            metadata,
            created_at: message.created_at(),
            sequence_number,
        })
    }
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

// ============================================================================
// Agent Session Models
// ============================================================================

/// Database row representation of an agent session.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = agent_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AgentSessionRow {
    /// Unique session identifier.
    pub id: Uuid,
    /// Reference to the containing conversation.
    pub conversation_id: Uuid,
    /// Agent backend identifier.
    pub agent_backend: String,
    /// First sequence number in this session.
    pub start_sequence: i64,
    /// Last sequence number (when session ends).
    pub end_sequence: Option<i64>,
    /// Turn IDs processed in this session as JSONB.
    pub turn_ids: Value,
    /// Handoff that initiated this session.
    pub initiated_by_handoff: Option<Uuid>,
    /// Handoff that terminated this session.
    pub terminated_by_handoff: Option<Uuid>,
    /// Context snapshots as JSONB.
    pub context_snapshots: Value,
    /// When the session started.
    pub started_at: DateTime<Utc>,
    /// When the session ended.
    pub ended_at: Option<DateTime<Utc>>,
    /// Session state.
    pub state: String,
}

/// Data for inserting a new agent session.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = agent_sessions)]
pub struct NewAgentSession {
    /// Unique session identifier.
    pub id: Uuid,
    /// Reference to the containing conversation.
    pub conversation_id: Uuid,
    /// Agent backend identifier.
    pub agent_backend: String,
    /// First sequence number in this session.
    pub start_sequence: i64,
    /// Last sequence number (when session ends).
    pub end_sequence: Option<i64>,
    /// Turn IDs processed in this session as JSONB.
    pub turn_ids: Value,
    /// Handoff that initiated this session.
    pub initiated_by_handoff: Option<Uuid>,
    /// Handoff that terminated this session.
    pub terminated_by_handoff: Option<Uuid>,
    /// Context snapshots as JSONB.
    pub context_snapshots: Value,
    /// When the session started.
    pub started_at: DateTime<Utc>,
    /// When the session ended.
    pub ended_at: Option<DateTime<Utc>>,
    /// Session state.
    pub state: String,
}

// ============================================================================
// Handoff Models
// ============================================================================

/// Database row representation of a handoff.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = handoffs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct HandoffRow {
    /// Unique handoff identifier.
    pub id: Uuid,
    /// Session being handed off from.
    pub source_session_id: Uuid,
    /// Conversation containing the handoff.
    pub conversation_id: Uuid,
    /// Session being handed off to.
    pub target_session_id: Option<Uuid>,
    /// Turn ID that triggered the handoff.
    pub prior_turn_id: Uuid,
    /// Tool calls that led to the handoff as JSONB.
    pub triggering_tool_calls: Value,
    /// Source agent backend identifier.
    pub source_agent: String,
    /// Target agent backend identifier.
    pub target_agent: String,
    /// Reason for the handoff.
    pub reason: Option<String>,
    /// When the handoff was initiated.
    pub initiated_at: DateTime<Utc>,
    /// When the handoff completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Handoff status.
    pub status: String,
}

/// Data for inserting a new handoff.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = handoffs)]
pub struct NewHandoff {
    /// Unique handoff identifier.
    pub id: Uuid,
    /// Session being handed off from.
    pub source_session_id: Uuid,
    /// Conversation containing the handoff.
    pub conversation_id: Uuid,
    /// Session being handed off to.
    pub target_session_id: Option<Uuid>,
    /// Turn ID that triggered the handoff.
    pub prior_turn_id: Uuid,
    /// Tool calls that led to the handoff as JSONB.
    pub triggering_tool_calls: Value,
    /// Source agent backend identifier.
    pub source_agent: String,
    /// Target agent backend identifier.
    pub target_agent: String,
    /// Reason for the handoff.
    pub reason: Option<String>,
    /// When the handoff was initiated.
    pub initiated_at: DateTime<Utc>,
    /// When the handoff completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Handoff status.
    pub status: String,
}

// ============================================================================
// Context Snapshot Models
// ============================================================================

/// Database row representation of a context snapshot.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = context_snapshots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ContextSnapshotRow {
    /// Unique snapshot identifier.
    pub id: Uuid,
    /// Reference to the containing conversation.
    pub conversation_id: Uuid,
    /// Reference to the agent session.
    pub session_id: Uuid,
    /// First sequence number in the context window.
    pub sequence_start: i64,
    /// Last sequence number in the context window.
    pub sequence_end: i64,
    /// Message counts by role as JSONB.
    pub message_summary: Value,
    /// Tool calls visible in the context window as JSONB.
    pub visible_tool_calls: Value,
    /// Token count estimate.
    pub token_estimate: Option<i64>,
    /// When the snapshot was captured.
    pub captured_at: DateTime<Utc>,
    /// Type of snapshot.
    pub snapshot_type: String,
}

/// Data for inserting a new context snapshot.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = context_snapshots)]
pub struct NewContextSnapshot {
    /// Unique snapshot identifier.
    pub id: Uuid,
    /// Reference to the containing conversation.
    pub conversation_id: Uuid,
    /// Reference to the agent session.
    pub session_id: Uuid,
    /// First sequence number in the context window.
    pub sequence_start: i64,
    /// Last sequence number in the context window.
    pub sequence_end: i64,
    /// Message counts by role as JSONB.
    pub message_summary: Value,
    /// Tool calls visible in the context window as JSONB.
    pub visible_tool_calls: Value,
    /// Token count estimate.
    pub token_estimate: Option<i64>,
    /// When the snapshot was captured.
    pub captured_at: DateTime<Utc>,
    /// Type of snapshot.
    pub snapshot_type: String,
}
