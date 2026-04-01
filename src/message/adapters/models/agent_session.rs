//! Diesel models for agent session persistence.
//!
//! Maps database rows to Rust structs for the `agent_sessions` table.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use super::super::schema::agent_sessions;

/// Database row representation of an agent session.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = agent_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AgentSessionRow {
    /// Unique session identifier.
    pub id: Uuid,
    /// Owning tenant identifier.
    pub tenant_id: Uuid,
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
    /// Owning tenant identifier.
    pub tenant_id: Uuid,
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
