//! Migration SQL fixtures for `PostgreSQL` integration tests.

/// SQL to create the base schema for tests.
pub const CREATE_SCHEMA_SQL: &str =
    include_str!("../../migrations/2026-01-15-000000_create_base_tables/up.sql");

/// SQL to add uniqueness constraints.
pub const ADD_CONSTRAINTS_SQL: &str =
    include_str!("../../migrations/2026-01-15-000001_add_message_uniqueness_constraints/up.sql");

/// SQL to add audit trigger.
pub const ADD_AUDIT_TRIGGER_SQL: &str =
    include_str!("../../migrations/2026-01-16-000000_add_audit_trigger/up.sql");

/// SQL to add agent sessions, handoffs, and context snapshots.
pub const ADD_HANDOFF_SCHEMA_SQL: &str =
    include_str!("../../migrations/2026-02-03-000000_add_agent_sessions_and_handoffs/up.sql");

/// SQL to add task lifecycle table.
pub const ADD_TASKS_SCHEMA_SQL: &str =
    include_str!("../../migrations/2026-02-09-000000_add_tasks_table/up.sql");

/// SQL to add branch and pull request lookup indexes for roadmap 1.2.2.
pub const ADD_BRANCH_PR_INDEXES_SQL: &str =
    include_str!("../../migrations/2026-02-11-000000_add_branch_pr_lookup_indexes/up.sql");

/// SQL to add agent backend registrations table for roadmap 1.3.1.
pub const ADD_BACKEND_REGISTRATIONS_SQL: &str =
    include_str!("../../migrations/2026-02-25-000000_add_backend_registrations_table/up.sql");

/// SQL to add MCP server registry table for roadmap 2.1.1.
pub const ADD_MCP_SERVERS_SQL: &str =
    include_str!("../../migrations/2026-02-28-000000_add_mcp_servers_table/up.sql");

/// SQL to add agent turn sessions table for roadmap 1.3.2.
pub const ADD_AGENT_TURN_SESSIONS_SQL: &str =
    include_str!("../../migrations/2026-03-03-000000_add_agent_turn_sessions_table/up.sql");

/// SQL to add tool catalog, audit log, and log metadata tables for roadmap 2.1.2.
pub const ADD_TOOL_CATALOG_SQL: &str =
    include_str!("../../migrations/2026-03-04-000000_add_tool_catalog_tables/up.sql");

/// SQL to add `tenant_id` to tool registry tables for tenant isolation.
pub const ADD_TENANT_ID_TO_TOOL_REGISTRY_SQL: &str =
    include_str!("../../migrations/2026-03-10-000000_add_tenant_id_to_tool_registry/up.sql");

/// SQL to add `tenant_id` to conversations and messages tables for tenant isolation.
pub const ADD_TENANT_ID_TO_CONVERSATIONS_AND_MESSAGES_SQL: &str = include_str!(
    "../../migrations/2026-04-01-000000_add_tenant_id_to_conversations_and_messages/up.sql"
);

/// SQL to enforce tenant-aware integrity for conversations and messages.
pub const ENFORCE_TENANT_SCOPE_FOR_CONVERSATIONS_AND_MESSAGES_SQL: &str = include_str!(
    "../../migrations/2026-04-01-000001_enforce_tenant_scope_for_conversations_and_messages/up.sql"
);

/// SQL to add hook execution log table for roadmap 2.3.1.
pub const ADD_HOOK_EXECUTIONS_SQL: &str =
    include_str!("../../migrations/2026-03-03-000000_add_hook_executions_table/up.sql");

/// SQL to tenant-scope hook execution log table for tenant isolation.
pub const ADD_TENANT_ID_TO_HOOK_EXECUTIONS_SQL: &str = include_str!(
    "../../migrations/2026-03-13-000000_add_tenant_id_to_hook_executions_table/up.sql"
);

/// SQL to enforce idempotent hook execution inserts per tenant and trigger context.
pub const ADD_HOOK_EXECUTIONS_UNIQUE_CONSTRAINT_SQL: &str =
    include_str!("../../migrations/2026-03-14-000000_add_hook_executions_unique_constraint/up.sql");

/// SQL to add hook policy audit projection storage and indexes.
pub const ADD_HOOK_POLICY_AUDIT_EVENTS_SQL: &str =
    include_str!("../../migrations/2026-03-22-000000_add_hook_policy_audit_events/up.sql");

/// SQL to enforce unique active agent session per conversation.
pub const ADD_UNIQUE_ACTIVE_SESSION_SQL: &str = include_str!(
    "../../migrations/2026-03-06-000000_add_unique_active_session_per_conversation/up.sql"
);

/// SQL to tenant-scope `mcp_servers` and enforce composite child foreign keys.
pub const ADD_TENANT_SCOPE_TO_MCP_SERVERS_SQL: &str =
    include_str!("../../migrations/2026-03-11-000000_tenant_scope_mcp_servers/up.sql");

/// SQL to tenant-scope agent backend and turn-session tables.
pub const ADD_TENANT_SCOPE_TO_AGENT_BACKEND_SQL: &str =
    include_str!("../../migrations/2026-03-13-000000_tenant_scope_agent_backend/up.sql");

/// SQL to allow reserved turn-session rows during atomic slot claims.
pub const ADD_RESERVED_TURN_SESSION_STATUS_SQL: &str = include_str!(
    "../../migrations/2026-03-20-000000_add_reserved_agent_turn_session_status/up.sql"
);

/// SQL to add tenant schema, tenant-aware uniqueness, and composite core FKs.
pub const ADD_TENANT_SCHEMA_AND_CONSTRAINTS_SQL: &str =
    include_str!("../../migrations/2026-03-21-000000_add_tenant_schema_and_constraints/up.sql");

/// Ordered migration registry used by the template database setup.
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("CREATE_SCHEMA_SQL", CREATE_SCHEMA_SQL),
    ("ADD_CONSTRAINTS_SQL", ADD_CONSTRAINTS_SQL),
    ("ADD_AUDIT_TRIGGER_SQL", ADD_AUDIT_TRIGGER_SQL),
    ("ADD_HANDOFF_SCHEMA_SQL", ADD_HANDOFF_SCHEMA_SQL),
    ("ADD_TASKS_SCHEMA_SQL", ADD_TASKS_SCHEMA_SQL),
    ("ADD_BRANCH_PR_INDEXES_SQL", ADD_BRANCH_PR_INDEXES_SQL),
    (
        "ADD_BACKEND_REGISTRATIONS_SQL",
        ADD_BACKEND_REGISTRATIONS_SQL,
    ),
    ("ADD_MCP_SERVERS_SQL", ADD_MCP_SERVERS_SQL),
    ("ADD_HOOK_EXECUTIONS_SQL", ADD_HOOK_EXECUTIONS_SQL),
    ("ADD_AGENT_TURN_SESSIONS_SQL", ADD_AGENT_TURN_SESSIONS_SQL),
    ("ADD_TOOL_CATALOG_SQL", ADD_TOOL_CATALOG_SQL),
    (
        "ADD_UNIQUE_ACTIVE_SESSION_SQL",
        ADD_UNIQUE_ACTIVE_SESSION_SQL,
    ),
    (
        "ADD_TENANT_ID_TO_TOOL_REGISTRY_SQL",
        ADD_TENANT_ID_TO_TOOL_REGISTRY_SQL,
    ),
    (
        "ADD_TENANT_SCOPE_TO_MCP_SERVERS_SQL",
        ADD_TENANT_SCOPE_TO_MCP_SERVERS_SQL,
    ),
    (
        "ADD_TENANT_ID_TO_HOOK_EXECUTIONS_SQL",
        ADD_TENANT_ID_TO_HOOK_EXECUTIONS_SQL,
    ),
    (
        "ADD_HOOK_EXECUTIONS_UNIQUE_CONSTRAINT_SQL",
        ADD_HOOK_EXECUTIONS_UNIQUE_CONSTRAINT_SQL,
    ),
    (
        "ADD_TENANT_SCOPE_TO_AGENT_BACKEND_SQL",
        ADD_TENANT_SCOPE_TO_AGENT_BACKEND_SQL,
    ),
    (
        "ADD_RESERVED_TURN_SESSION_STATUS_SQL",
        ADD_RESERVED_TURN_SESSION_STATUS_SQL,
    ),
    (
        "ADD_TENANT_SCHEMA_AND_CONSTRAINTS_SQL",
        ADD_TENANT_SCHEMA_AND_CONSTRAINTS_SQL,
    ),
    (
        "ADD_HOOK_POLICY_AUDIT_EVENTS_SQL",
        ADD_HOOK_POLICY_AUDIT_EVENTS_SQL,
    ),
    (
        "ADD_TENANT_ID_TO_CONVERSATIONS_AND_MESSAGES_SQL",
        ADD_TENANT_ID_TO_CONVERSATIONS_AND_MESSAGES_SQL,
    ),
    (
        "ENFORCE_TENANT_SCOPE_FOR_CONVERSATIONS_AND_MESSAGES_SQL",
        ENFORCE_TENANT_SCOPE_FOR_CONVERSATIONS_AND_MESSAGES_SQL,
    ),
];
