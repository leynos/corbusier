//! `PostgreSQL` integration tests for the message repository.
//!
//! Tests are organized into modules by functionality:
//! - `crud_tests`: Basic CRUD operations
//! - `sequence_tests`: Sequence number management
//! - `uniqueness_tests`: Uniqueness constraint enforcement
//! - `serialization_tests`: Role parsing, JSONB round-trips, audit context

#![expect(
    clippy::print_stderr,
    reason = "Test cleanup warnings are informational"
)]

mod postgres {
    pub mod helpers;

    mod audit_tests;
    mod crud_tests;
    mod sequence_tests;
    mod serialization_tests;
    mod uniqueness_tests;
}
