//! `PostgreSQL` integration tests for the message, task, and backend registry repositories.
//!
//! Tests are organized into modules by functionality:
//! - `cluster`: Embedded `PostgreSQL` cluster lifecycle helpers
//! - `agent_session_tests`: Agent session persistence and active-session uniqueness
//! - `audit_tests`: Audit context capture and verification
//! - `agent_turn_orchestration_tests`: Turn execution and session continuity
//! - `backend_registry_tests`: Agent backend registration and discovery
//! - `crud_tests`: Basic CRUD operations
//! - `mcp_server_lifecycle_tests`: MCP server lifecycle persistence
//! - `sequence_tests`: Sequence number management
//! - `serialization_tests`: Role parsing, JSONB round-trips, metadata handling
//! - `slash_command_tests`: Slash command metadata round-trips
//! - `sql_helpers_tests`: SQL helper function unit tests
//! - `task_branch_pr_postgres_tests`: Branch and PR association tests
//! - `task_lifecycle_tests`: Issue-to-task creation and lookup
//! - `task_tenant_isolation_tests`: Tenant context propagation for task operations
//! - `tenant_schema_constraints_tests`: Composite FK enforcement for tenant-aware core tables
//! - `tool_discovery_tenant_isolation_tests`: Composite FK and index-plan checks
//! - `uniqueness_tests`: Uniqueness constraint enforcement
//! - `tool_discovery_routing_tests`: Tool discovery, catalog, and audit trail
//! - `tool_policy_enforcement_tests`: Hook-backed policy enforcement for tool calls
//! - `hook_engine_tests`: Hook execution log persistence
//! - `http_api_surface_tests`: HTTP API surface integration tests

mod http_api_test_helpers;
mod test_helpers;
mod worker_locator;

mod postgres {
    //! Groups the `PostgreSQL` integration suites behind a shared module root.

    pub mod cluster;
    pub mod helpers;

    mod agent_session_tests;
    mod agent_turn_orchestration_tests;
    mod audit_tests;
    mod backend_registry_tests;
    mod crud_tests;
    mod hook_engine_tests;
    mod http_api_surface_tests;
    mod mcp_server_lifecycle_tests;
    mod sequence_tests;
    mod serialization_tests;
    mod slash_command_tests;
    mod sql_helpers_tests;
    mod task_branch_pr_postgres_tests;
    mod task_lifecycle_tests;
    mod task_tenant_isolation_tests;
    mod tenant_schema_constraints_tests;
    mod tool_discovery_routing_tests;
    mod tool_discovery_tenant_isolation_tests;
    mod tool_policy_enforcement_tests;
    mod uniqueness_tests;
}
