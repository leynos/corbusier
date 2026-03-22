//! Diesel models for hook policy audit persistence.

use super::schema::hook_policy_audit_events;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

/// Row representation for a hook policy audit record.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = hook_policy_audit_events)]
pub struct PolicyAuditEventRow {
    /// Event identifier.
    pub id: uuid::Uuid,
    /// Tenant identifier.
    pub tenant_id: uuid::Uuid,
    /// Hook execution identifier.
    pub hook_execution_id: uuid::Uuid,
    /// Trigger context identifier.
    pub trigger_context_id: uuid::Uuid,
    /// Trigger type string.
    pub trigger_type: String,
    /// Hook identifier string.
    pub hook_id: String,
    /// Action identifier string.
    pub action_id: String,
    /// Correlated task identifier, if any.
    pub task_id: Option<uuid::Uuid>,
    /// Correlated conversation identifier, if any.
    pub conversation_id: Option<uuid::Uuid>,
    /// Policy decision string.
    pub decision: String,
    /// Structured violation JSON payload, if any.
    pub violation: Option<serde_json::Value>,
    /// Raw policy payload.
    pub payload: serde_json::Value,
    /// Projection timestamp.
    pub recorded_at: DateTime<Utc>,
}

/// Insertable row for a hook policy audit record.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = hook_policy_audit_events)]
pub struct NewPolicyAuditEventRow {
    /// Event identifier.
    pub id: uuid::Uuid,
    /// Tenant identifier.
    pub tenant_id: uuid::Uuid,
    /// Hook execution identifier.
    pub hook_execution_id: uuid::Uuid,
    /// Trigger context identifier.
    pub trigger_context_id: uuid::Uuid,
    /// Trigger type string.
    pub trigger_type: String,
    /// Hook identifier string.
    pub hook_id: String,
    /// Action identifier string.
    pub action_id: String,
    /// Correlated task identifier, if any.
    pub task_id: Option<uuid::Uuid>,
    /// Correlated conversation identifier, if any.
    pub conversation_id: Option<uuid::Uuid>,
    /// Policy decision string.
    pub decision: String,
    /// Structured violation JSON payload, if any.
    pub violation: Option<serde_json::Value>,
    /// Raw policy payload.
    pub payload: serde_json::Value,
    /// Projection timestamp.
    pub recorded_at: DateTime<Utc>,
}
