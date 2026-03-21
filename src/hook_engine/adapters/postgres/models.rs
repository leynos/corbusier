//! Diesel models for hook execution persistence.

use super::schema::hook_executions;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

/// Row representation for a hook execution record.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = hook_executions)]
pub struct HookExecutionRow {
    /// Execution identifier.
    pub id: uuid::Uuid,
    /// Tenant identifier.
    pub tenant_id: uuid::Uuid,
    /// Trigger context identifier.
    pub trigger_context_id: uuid::Uuid,
    /// Hook identifier string.
    pub hook_id: String,
    /// Trigger type string.
    pub trigger_type: String,
    /// Predicate data JSON payload.
    pub predicate_data: serde_json::Value,
    /// Action results JSON payload.
    pub action_results: serde_json::Value,
    /// Execution status string.
    pub status: String,
    /// Execution timestamp.
    pub executed_at: DateTime<Utc>,
}

/// Insertable row for a hook execution record.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = hook_executions)]
pub struct NewHookExecutionRow {
    /// Execution identifier.
    pub id: uuid::Uuid,
    /// Tenant identifier.
    pub tenant_id: uuid::Uuid,
    /// Trigger context identifier.
    pub trigger_context_id: uuid::Uuid,
    /// Hook identifier string.
    pub hook_id: String,
    /// Trigger type string.
    pub trigger_type: String,
    /// Predicate data JSON payload.
    pub predicate_data: serde_json::Value,
    /// Action results JSON payload.
    pub action_results: serde_json::Value,
    /// Execution status string.
    pub status: String,
    /// Execution timestamp.
    pub executed_at: DateTime<Utc>,
}
