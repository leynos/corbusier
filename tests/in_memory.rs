//! In-memory repository integration tests.
//!
//! Tests are organized into modules by functionality:
//! - `conversation_flow_tests`: Message ordering, role preservation, retrieval
//! - `sequence_tests`: Sequence number management
//! - `constraint_tests`: Duplicate detection, exists checks
//! - `handoff_tests`: Agent handoff workflow
//! - `task_lifecycle_tests`: Issue-to-task creation and tracking

mod in_memory {
    pub mod helpers;

    mod constraint_tests;
    mod conversation_flow_tests;
    mod handoff_tests;
    mod sequence_tests;
    mod slash_command_tests;
    mod task_lifecycle_tests;
}
