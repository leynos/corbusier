//! Diesel schema for MCP server registry persistence.

diesel::table! {
    /// MCP server registry records.
    mcp_servers (id) {
        /// Internal server identifier.
        id -> Uuid,
        /// Unique human-readable server name.
        #[max_length = 100]
        name -> Varchar,
        /// Transport configuration as JSONB.
        transport -> Jsonb,
        /// Lifecycle state (`registered`, `running`, `stopped`).
        #[max_length = 50]
        lifecycle_state -> Varchar,
        /// Health status (`unknown`, `healthy`, `unhealthy`).
        #[max_length = 50]
        health_status -> Varchar,
        /// Optional health message for unhealthy states.
        health_message -> Nullable<Text>,
        /// Optional timestamp of the last health check.
        health_checked_at -> Nullable<Timestamptz>,
        /// Creation timestamp.
        created_at -> Timestamptz,
        /// Last update timestamp.
        updated_at -> Timestamptz,
    }
}
