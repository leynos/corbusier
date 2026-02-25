//! Diesel row models for agent backend registration persistence.

use super::schema::backend_registrations;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;

/// Query result row for backend registration records.
#[derive(Debug, Clone, Queryable, QueryableByName, Selectable)]
#[diesel(table_name = backend_registrations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BackendRegistrationRow {
    /// Internal backend identifier.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: uuid::Uuid,
    /// Unique human-readable backend name.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub name: String,
    /// Lifecycle status.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub status: String,
    /// Capability metadata JSON payload.
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub capabilities: Value,
    /// Provider information JSON payload.
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub backend_info: Value,
    /// Creation timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
}

/// Insert model for backend registration records.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = backend_registrations)]
pub struct NewBackendRegistrationRow {
    /// Internal backend identifier.
    pub id: uuid::Uuid,
    /// Unique human-readable backend name.
    pub name: String,
    /// Lifecycle status.
    pub status: String,
    /// Capability metadata JSON payload.
    pub capabilities: Value,
    /// Provider information JSON payload.
    pub backend_info: Value,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
