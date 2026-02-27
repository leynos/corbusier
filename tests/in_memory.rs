//! In-memory repository integration tests.
//!
//! Tests are organized into modules by functionality:
//! - `conversation_flow_tests`: Message ordering, role preservation, retrieval
//! - `sequence_tests`: Sequence number management
//! - `constraint_tests`: Duplicate detection, exists checks
//! - `handoff_tests`: Agent handoff workflow
//! - `task_lifecycle_tests`: Issue-to-task creation and tracking
//! - `backend_registry_tests`: Agent backend registration and discovery

mod in_memory {
    pub mod helpers;

    mod backend_registry_tests;
    mod constraint_tests;
    mod conversation_flow_tests;
    mod handoff_tests;
    mod sequence_tests;
    mod task_lifecycle_tests;
}
