//! Diesel models for handoff persistence.
//!
//! Maps database rows to Rust structs for the `handoffs` table.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use super::super::schema::handoffs;

/// Database row representation of a handoff.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = handoffs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct HandoffRow {
    /// Unique handoff identifier.
    pub id: Uuid,
    /// Owning tenant identifier.
    pub tenant_id: Uuid,
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
    /// Owning tenant identifier.
    pub tenant_id: Uuid,
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
