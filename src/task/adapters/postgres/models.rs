//! Diesel row models for task persistence.

use super::schema::tasks;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;

/// Query result row for task records.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = tasks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TaskRow {
    /// Internal task identifier.
    pub id: uuid::Uuid,
    /// Origin JSON payload.
    pub origin: Value,
    /// Lifecycle state.
    pub state: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Insert model for task records.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = tasks)]
pub struct NewTaskRow {
    /// Internal task identifier.
    pub id: uuid::Uuid,
    /// Origin JSON payload.
    pub origin: Value,
    /// Lifecycle state.
    pub state: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
