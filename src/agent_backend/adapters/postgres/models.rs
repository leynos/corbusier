//! Diesel row models for agent backend orchestration persistence.

use super::schema::{agent_turn_sessions, backend_registrations};
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
    /// Tenant identifier owning this backend.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub tenant_id: uuid::Uuid,
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
    /// Tenant identifier owning this backend.
    pub tenant_id: uuid::Uuid,
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

/// Query result row for turn-session records.
#[derive(Debug, Clone, Queryable, QueryableByName, Selectable)]
#[diesel(table_name = agent_turn_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AgentTurnSessionRow {
    /// Internal session identifier.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: uuid::Uuid,
    /// Tenant identifier owning this session.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub tenant_id: uuid::Uuid,
    /// Owning backend registration identifier.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub backend_id: uuid::Uuid,
    /// Conversation identifier.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub conversation_id: uuid::Uuid,
    /// Backend-native runtime session identifier.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub runtime_session_id: String,
    /// Lifecycle status.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub status: String,
    /// Session TTL in seconds.
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub ttl_seconds: i64,
    /// Session start timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub started_at: DateTime<Utc>,
    /// Last successful turn timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub last_used_at: DateTime<Utc>,
    /// Session expiry timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub expires_at: DateTime<Utc>,
    /// Session end timestamp.
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub ended_at: Option<DateTime<Utc>>,
    /// Number of successful turns in the session.
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub turn_count: i64,
}

/// Insert model for turn-session records.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = agent_turn_sessions)]
pub struct NewAgentTurnSessionRow {
    /// Internal session identifier.
    pub id: uuid::Uuid,
    /// Tenant identifier owning this session.
    pub tenant_id: uuid::Uuid,
    /// Owning backend registration identifier.
    pub backend_id: uuid::Uuid,
    /// Conversation identifier.
    pub conversation_id: uuid::Uuid,
    /// Backend-native runtime session identifier.
    pub runtime_session_id: String,
    /// Lifecycle status.
    pub status: String,
    /// Session TTL in seconds.
    pub ttl_seconds: i64,
    /// Session start timestamp.
    pub started_at: DateTime<Utc>,
    /// Last successful turn timestamp.
    pub last_used_at: DateTime<Utc>,
    /// Session expiry timestamp.
    pub expires_at: DateTime<Utc>,
    /// Session end timestamp.
    pub ended_at: Option<DateTime<Utc>>,
    /// Number of successful turns in the session.
    pub turn_count: i64,
}
