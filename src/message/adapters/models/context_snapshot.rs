//! Diesel models for context snapshot persistence.
//!
//! Maps database rows to Rust structs for the `context_snapshots` table.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use super::super::schema::context_snapshots;

/// Database row representation of a context snapshot.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = context_snapshots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ContextSnapshotRow {
    /// Unique snapshot identifier.
    pub id: Uuid,
    /// Owning tenant identifier.
    pub tenant_id: Uuid,
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
    /// Owning tenant identifier.
    pub tenant_id: Uuid,
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
