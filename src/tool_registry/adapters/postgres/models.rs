//! Diesel row models for MCP server registry persistence.

use super::schema::mcp_servers;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;

/// Query result row for MCP server records.
#[derive(Debug, Clone, Queryable, QueryableByName, Selectable)]
#[diesel(table_name = mcp_servers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct McpServerRow {
    /// Internal server identifier.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: uuid::Uuid,
    /// Unique server name.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub name: String,
    /// Transport configuration payload.
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub transport: Value,
    /// Lifecycle state.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub lifecycle_state: String,
    /// Health status.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub health_status: String,
    /// Optional health message.
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub health_message: Option<String>,
    /// Optional health check timestamp.
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub health_checked_at: Option<DateTime<Utc>>,
    /// Creation timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
}

/// Insert model for MCP server records.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = mcp_servers)]
pub struct NewMcpServerRow {
    /// Internal server identifier.
    pub id: uuid::Uuid,
    /// Unique server name.
    pub name: String,
    /// Transport configuration payload.
    pub transport: Value,
    /// Lifecycle state.
    pub lifecycle_state: String,
    /// Health status.
    pub health_status: String,
    /// Optional health message.
    pub health_message: Option<String>,
    /// Optional health check timestamp.
    pub health_checked_at: Option<DateTime<Utc>>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}
