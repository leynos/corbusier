//! In-memory repository integration tests.
//!
//! Tests are organized into modules by functionality:
//! - `conversation_flow_tests`: Message ordering, role preservation, retrieval
//! - `sequence_tests`: Sequence number management
//! - `constraint_tests`: Duplicate detection, exists checks
//! - `handoff_tests`: Agent handoff workflow
//! - `task_lifecycle_tests`: Issue-to-task creation and tracking
//! - `backend_registry_tests`: Agent backend registration and discovery
//! - `mcp_server_lifecycle_tests`: MCP server registration and lifecycle
//! - `tool_discovery_routing_tests`: Tool discovery, catalog, and call routing
//! - `hook_engine_tests`: Hook execution and persistence
//! - `agent_turn_orchestration_tests`: Turn execution and session continuity

mod in_memory {
    //! Groups the in-memory integration suites behind a shared module root.

    pub mod helpers;

    mod agent_turn_orchestration_tests;
    mod backend_registry_tests;
    mod constraint_tests;
    mod conversation_flow_tests;
    mod handoff_tests;
    mod hook_engine_tests;
    mod mcp_server_lifecycle_tests;
    mod sequence_tests;
    mod slash_command_tests;
    mod task_lifecycle_tests;
    mod tool_discovery_routing_tests;
}
