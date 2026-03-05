//! Diesel row models for tool catalog, audit log, and log metadata tables.

use super::catalog_schema::{mcp_tool_catalog, tool_call_audit_log, tool_log_metadata};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;

/// Query result row for tool catalog records.
#[derive(Debug, Clone, Queryable, QueryableByName, Selectable)]
#[diesel(table_name = mcp_tool_catalog)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CatalogEntryRow {
    /// Catalog entry identifier.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: uuid::Uuid,
    /// Owning MCP server identifier.
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub server_id: uuid::Uuid,
    /// Server name at discovery time.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub server_name: String,
    /// Unique tool name.
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub tool_name: String,
    /// Tool description.
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub tool_description: String,
    /// Input schema as JSONB.
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub input_schema: Value,
    /// Output schema as JSONB (nullable).
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Jsonb>)]
    pub output_schema: Option<Value>,
    /// Whether the tool is currently available.
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub available: bool,
    /// Discovery timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub discovered_at: DateTime<Utc>,
    /// Last update timestamp.
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
}

/// Insert model for tool catalog records.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = mcp_tool_catalog)]
pub struct NewCatalogEntryRow {
    /// Catalog entry identifier.
    pub id: uuid::Uuid,
    /// Owning MCP server identifier.
    pub server_id: uuid::Uuid,
    /// Server name at discovery time.
    pub server_name: String,
    /// Unique tool name.
    pub tool_name: String,
    /// Tool description.
    pub tool_description: String,
    /// Input schema as JSONB.
    pub input_schema: Value,
    /// Output schema as JSONB (nullable).
    pub output_schema: Option<Value>,
    /// Whether the tool is currently available.
    pub available: bool,
    /// Discovery timestamp.
    pub discovered_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Insert model for audit log records.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = tool_call_audit_log)]
pub struct NewAuditLogRow {
    /// Audit record identifier.
    pub id: uuid::Uuid,
    /// Tool call invocation identifier.
    pub call_id: uuid::Uuid,
    /// Tool name invoked.
    pub tool_name: String,
    /// Server that handled the call.
    pub server_id: uuid::Uuid,
    /// Call parameters as JSONB.
    pub parameters: Value,
    /// Outcome (`success` or `failure`).
    pub outcome: String,
    /// Outcome content (for success).
    pub outcome_content: Option<Value>,
    /// Outcome error message (for failure).
    pub outcome_error: Option<String>,
    /// Call duration in milliseconds.
    pub duration_ms: i64,
    /// Call initiation timestamp.
    pub initiated_at: DateTime<Utc>,
    /// Call completion timestamp.
    pub completed_at: DateTime<Utc>,
    /// Object store path to captured stderr.
    pub stderr_log_path: Option<String>,
}

/// Insert model for log metadata records.
#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = tool_log_metadata)]
pub struct NewLogMetadataRow {
    /// Log entry identifier.
    pub id: uuid::Uuid,
    /// Owning MCP server identifier.
    pub server_id: uuid::Uuid,
    /// Log kind (`startup` or `tool_call`).
    pub kind: String,
    /// Associated tool call identifier (nullable).
    pub call_id: Option<uuid::Uuid>,
    /// Object store path.
    pub object_path: String,
    /// Size of the log blob in bytes.
    pub byte_count: i64,
    /// Capture timestamp.
    pub captured_at: DateTime<Utc>,
    /// Expiry timestamp.
    pub expires_at: DateTime<Utc>,
}
