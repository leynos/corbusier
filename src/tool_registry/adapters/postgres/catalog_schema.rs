//! Diesel schema for tool catalog, audit log, and log metadata tables.

diesel::table! {
    /// Tool catalog records discovered from MCP servers.
    mcp_tool_catalog (id) {
        /// Catalog entry identifier.
        id -> Uuid,
        /// Owning tenant identifier.
        tenant_id -> Uuid,
        /// Owning MCP server identifier.
        server_id -> Uuid,
        /// Server name at discovery time.
        #[max_length = 100]
        server_name -> Varchar,
        /// Tool name (unique per tenant, see `idx_mcp_tool_catalog_tenant_tool_name`).
        #[max_length = 255]
        tool_name -> Varchar,
        /// Tool description.
        tool_description -> Text,
        /// Input schema as JSONB.
        input_schema -> Jsonb,
        /// Output schema as JSONB (nullable).
        output_schema -> Nullable<Jsonb>,
        /// Whether the tool is currently available.
        available -> Bool,
        /// Discovery timestamp.
        discovered_at -> Timestamptz,
        /// Last update timestamp.
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    /// Immutable audit records for tool call invocations.
    tool_call_audit_log (id) {
        /// Audit record identifier.
        id -> Uuid,
        /// Owning tenant identifier.
        tenant_id -> Uuid,
        /// Tool call invocation identifier.
        call_id -> Uuid,
        /// Tool name invoked.
        #[max_length = 255]
        tool_name -> Varchar,
        /// Server that handled the call.
        server_id -> Uuid,
        /// Call parameters as JSONB.
        parameters -> Jsonb,
        /// Outcome (`success` or `failure`).
        #[max_length = 50]
        outcome -> Varchar,
        /// Outcome content (for success).
        outcome_content -> Nullable<Jsonb>,
        /// Outcome error message (for failure).
        outcome_error -> Nullable<Text>,
        /// Call duration in milliseconds.
        duration_ms -> Int8,
        /// Call initiation timestamp.
        initiated_at -> Timestamptz,
        /// Call completion timestamp.
        completed_at -> Timestamptz,
        /// Object store path to captured stderr log (nullable).
        #[max_length = 512]
        stderr_log_path -> Nullable<Varchar>,
    }
}

diesel::table! {
    /// Metadata index for stderr log blobs stored in `object_store`.
    tool_log_metadata (id) {
        /// Log entry identifier.
        id -> Uuid,
        /// Owning tenant identifier.
        tenant_id -> Uuid,
        /// Owning MCP server identifier.
        server_id -> Uuid,
        /// Log kind (`startup` or `tool_call`).
        #[max_length = 50]
        kind -> Varchar,
        /// Associated tool call identifier (nullable, set for `tool_call` kind).
        call_id -> Nullable<Uuid>,
        /// Object store path to the log blob.
        #[max_length = 512]
        object_path -> Varchar,
        /// Size of the log blob in bytes.
        byte_count -> Int8,
        /// Capture timestamp.
        captured_at -> Timestamptz,
        /// Expiry timestamp for retention sweeps.
        expires_at -> Timestamptz,
    }
}
