//! In-memory repository integration tests.
//!
//! Tests are organized into modules by functionality:
//! - `conversation_flow_tests`: Message ordering, role preservation, retrieval
//! - `sequence_tests`: Sequence number management
//! - `constraint_tests`: Duplicate detection, exists checks

#![expect(
    clippy::panic_in_result_fn,
    reason = "Test functions use assertions for verification while returning Result for error propagation"
)]

mod in_memory {
    pub mod helpers;

    mod constraint_tests;
    mod conversation_flow_tests;
    mod sequence_tests;
}
