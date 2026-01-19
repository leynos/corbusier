//! Unit tests for `RepositoryError` and error conversions.

use crate::message::error::RepositoryError;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

// ============================================================================
// From<diesel::result::Error> for RepositoryError tests
// ============================================================================

#[test]
fn repository_error_from_diesel_not_found() {
    let diesel_err = DieselError::NotFound;
    let repo_err = RepositoryError::from(diesel_err);

    // Should produce a Database variant
    assert!(matches!(repo_err, RepositoryError::Database(_)));
    assert!(repo_err.to_string().contains("database error"));
}

#[test]
fn repository_error_from_diesel_rollback_transaction() {
    let diesel_err = DieselError::RollbackTransaction;
    let repo_err = RepositoryError::from(diesel_err);

    assert!(matches!(repo_err, RepositoryError::Database(_)));
}

#[test]
fn repository_error_from_diesel_query_builder_error() {
    let diesel_err = DieselError::QueryBuilderError("test error".into());
    let repo_err = RepositoryError::from(diesel_err);

    assert!(matches!(repo_err, RepositoryError::Database(_)));
    let display = repo_err.to_string();
    assert!(display.contains("database error"));
}

#[test]
fn repository_error_from_diesel_database_error_unique_violation() {
    // Create a mock database error for unique constraint violation
    let db_err = DieselError::DatabaseError(
        DatabaseErrorKind::UniqueViolation,
        Box::new("duplicate key value".to_owned()),
    );
    let repo_err = RepositoryError::from(db_err);

    assert!(matches!(repo_err, RepositoryError::Database(_)));
}

#[test]
fn repository_error_from_diesel_database_error_foreign_key() {
    let db_err = DieselError::DatabaseError(
        DatabaseErrorKind::ForeignKeyViolation,
        Box::new("foreign key constraint".to_owned()),
    );
    let repo_err = RepositoryError::from(db_err);

    assert!(matches!(repo_err, RepositoryError::Database(_)));
}

// ============================================================================
// RepositoryError helper method tests
// ============================================================================

#[test]
fn repository_error_database_helper_wraps_error() {
    let io_err = std::io::Error::other("io error");
    let repo_err = RepositoryError::database(io_err);

    assert!(matches!(repo_err, RepositoryError::Database(_)));
    assert!(repo_err.to_string().contains("database error"));
}

#[test]
fn repository_error_serialization_helper() {
    let repo_err = RepositoryError::serialization("failed to parse");

    assert!(matches!(repo_err, RepositoryError::Serialization(_)));
    assert!(repo_err.to_string().contains("failed to parse"));
}

#[test]
fn repository_error_connection_helper() {
    let repo_err = RepositoryError::connection("connection refused");

    assert!(matches!(repo_err, RepositoryError::Connection(_)));
    assert!(repo_err.to_string().contains("connection refused"));
}

// ============================================================================
// Display trait tests
// ============================================================================

#[test]
fn repository_error_display_database() {
    let diesel_err = DieselError::NotFound;
    let repo_err = RepositoryError::from(diesel_err);
    let display = format!("{repo_err}");

    assert!(display.starts_with("database error:"));
}

#[test]
fn repository_error_display_serialization() {
    let repo_err = RepositoryError::serialization("invalid JSON");
    let display = format!("{repo_err}");

    assert!(display.starts_with("serialization error:"));
    assert!(display.contains("invalid JSON"));
}

#[test]
fn repository_error_display_connection() {
    let repo_err = RepositoryError::connection("timeout");
    let display = format!("{repo_err}");

    assert!(display.starts_with("connection error:"));
    assert!(display.contains("timeout"));
}
