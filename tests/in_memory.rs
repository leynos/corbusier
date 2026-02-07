//! In-memory repository integration tests.
//!
//! Tests are organized into modules by functionality:
//! - `conversation_flow_tests`: Message ordering, role preservation, retrieval
//! - `sequence_tests`: Sequence number management
//! - `constraint_tests`: Duplicate detection, exists checks
//! - `handoff_tests`: Agent handoff workflow

mod in_memory {
    pub mod helpers;

    mod constraint_tests;
    mod conversation_flow_tests;
    mod handoff_tests;
    mod sequence_tests;
}
