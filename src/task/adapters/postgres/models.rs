//! Diesel row models for task persistence.

use super::schema::tasks;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;

/// Query result row for task records.
#[derive(Debug, Clone, Queryable, QueryableByName, Selectable)]
#[diesel(table_name = tasks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TaskRow {
    /// Internal task identifier.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: uuid::Uuid,
    /// Origin JSON payload.
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub origin: Value,
    /// Optional branch reference reserved for roadmap item 1.2.2.
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub branch_ref: Option<String>,
    /// Optional pull-request reference reserved for roadmap item 1.2.2.
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
    pub pull_request_ref: Option<String>,
    /// Lifecycle state.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub state: String,
    /// Optional workspace identifier reserved for roadmap item 1.2.3.
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    pub workspace_id: Option<uuid::Uuid>,
    /// Creation timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
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
    /// Optional branch reference reserved for roadmap item 1.2.2.
    pub branch_ref: Option<String>,
    /// Optional pull-request reference reserved for roadmap item 1.2.2.
    pub pull_request_ref: Option<String>,
    /// Lifecycle state.
    pub state: String,
    /// Optional workspace identifier reserved for roadmap item 1.2.3.
    pub workspace_id: Option<uuid::Uuid>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
