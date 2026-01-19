//! SQL execution helpers for `PostgreSQL` repository.
//!
//! Contains database insert operations, constraint error mapping, and
//! session variable management for audit context.

use diesel::prelude::*;

use super::super::audit_context::AuditContext;
use super::super::models::NewMessage;
use super::super::schema::messages;
use crate::message::{
    domain::{ConversationId, MessageId, SequenceNumber},
    error::RepositoryError,
    ports::repository::RepositoryResult,
};

/// Identifiers needed for semantic error mapping in insert operations.
pub(super) struct InsertIds {
    pub msg_id: MessageId,
    pub conv_id: ConversationId,
    pub seq_num: SequenceNumber,
}

/// Inserts a message into the database.
///
/// Maps constraint violations to semantic error types when possible.
/// Pre-checks in `store()` should catch most duplicates with proper IDs,
/// but this handles race conditions where the constraint catches duplicates
/// that slipped past the pre-check.
pub(super) fn insert_message(
    conn: &mut PgConnection,
    new_message: &NewMessage,
    ids: &InsertIds,
) -> RepositoryResult<()> {
    diesel::insert_into(messages::table)
        .values(new_message)
        .execute(conn)
        .map_err(|e| map_insert_error(e, ids))?;
    Ok(())
}

/// Maps Diesel errors to semantic repository errors.
///
/// Inspects unique constraint violations to determine if they represent
/// duplicate message IDs or duplicate sequence numbers, returning the
/// appropriate error variant with the relevant identifiers.
fn map_insert_error(err: diesel::result::Error, ids: &InsertIds) -> RepositoryError {
    use diesel::result::DatabaseErrorKind;
    let diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, info) = &err
    else {
        return RepositoryError::database(err);
    };

    let Some(constraint) = info.constraint_name() else {
        return RepositoryError::database(err);
    };

    map_constraint_to_duplicate_error(constraint, ids)
        .unwrap_or_else(|| RepositoryError::database(err))
}

/// Maps a constraint name to a semantic duplicate error.
///
/// Returns `Some(RepositoryError)` if the constraint represents a known
/// duplicate violation, `None` otherwise.
fn map_constraint_to_duplicate_error(constraint: &str, ids: &InsertIds) -> Option<RepositoryError> {
    match constraint {
        "messages_id_unique" | "messages_pkey" => {
            Some(RepositoryError::DuplicateMessage(ids.msg_id))
        }
        "messages_conversation_sequence_unique" => Some(RepositoryError::DuplicateSequence {
            conversation_id: ids.conv_id,
            sequence: ids.seq_num,
        }),
        _ => None,
    }
}

/// Sets a single `PostgreSQL` session variable for audit context.
///
/// The key name is interpolated via `format!` but is always a controlled
/// static string from the audit context fields, not user input.
/// `PostgreSQL` does not support parameterized identifiers or values in
/// SET statements, so both key and value must be interpolated.
///
/// # Security
///
/// UUID values are formatted using their canonical hyphenated representation
/// which contains only hexadecimal digits and hyphens, making SQL injection
/// impossible.
fn set_session_uuid(conn: &mut PgConnection, key: &str, value: uuid::Uuid) -> RepositoryResult<()> {
    // PostgreSQL SET does not support parameter binding ($1).
    // UUID hyphenated format (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx) contains
    // only hex digits and hyphens, so it is safe to interpolate directly.
    diesel::sql_query(format!("SET LOCAL app.{key} = '{value}'"))
        .execute(conn)
        .map_err(RepositoryError::database)?;
    Ok(())
}

/// Sets `PostgreSQL` session variables for audit context.
///
/// Each audit field is set via [`set_session_uuid`], which interpolates UUID
/// values directly into the SET statement. This is safe because UUID formatting
/// produces only hexadecimal digits and hyphens, preventing SQL injection.
pub(super) fn set_audit_context(
    conn: &mut PgConnection,
    audit: &AuditContext,
) -> RepositoryResult<()> {
    if let Some(correlation_id) = audit.correlation_id {
        set_session_uuid(conn, "correlation_id", correlation_id)?;
    }
    if let Some(causation_id) = audit.causation_id {
        set_session_uuid(conn, "causation_id", causation_id)?;
    }
    if let Some(user_id) = audit.user_id {
        set_session_uuid(conn, "user_id", user_id)?;
    }
    if let Some(session_id) = audit.session_id {
        set_session_uuid(conn, "session_id", session_id)?;
    }
    Ok(())
}
