//! `PostgreSQL` integration tests for the message and task repositories.
//!
//! Tests are organized into modules by functionality:
//! - `cluster`: Embedded `PostgreSQL` cluster lifecycle helpers
//! - `audit_tests`: Audit context capture and verification
//! - `crud_tests`: Basic CRUD operations
//! - `sequence_tests`: Sequence number management
//! - `serialization_tests`: Role parsing, JSONB round-trips, metadata handling
//! - `sql_helpers_tests`: SQL helper function unit tests
//! - `task_branch_pr_postgres_tests`: Branch and PR association tests
//! - `task_lifecycle_tests`: Issue-to-task creation and lookup
//! - `uniqueness_tests`: Uniqueness constraint enforcement

mod test_helpers;
mod worker_locator;

mod postgres {
    pub mod cluster;
    pub mod helpers;

    mod audit_tests;
    mod crud_tests;
    mod sequence_tests;
    mod serialization_tests;
    mod sql_helpers_tests;
    mod task_branch_pr_postgres_tests;
    mod task_lifecycle_tests;
    mod uniqueness_tests;
}
