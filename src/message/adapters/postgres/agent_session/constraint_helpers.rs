//! Constraint checking and error mapping for the agent session `PostgreSQL` adapter.
//!
//! Maps database-level constraint violations (unique index, primary key) to
//! domain error variants and provides the application-level active-session check.

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorInformation, DatabaseErrorKind, Error as DieselError};

use crate::message::{
    adapters::schema::agent_sessions,
    domain::{AgentSessionId, AgentSessionState, ConversationId},
    ports::agent_session::{SessionError, SessionResult},
};

use super::super::tenant_tx::{FromTxError, TxError};

/// Name of the partial unique index enforcing one active session per conversation.
const ACTIVE_SESSION_UNIQUE_INDEX: &str = "idx_agent_sessions_one_active_per_conversation";

impl FromTxError<Self> for SessionError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(e) => e,
            TxError::Diesel(e) => Self::persistence(e),
        }
    }
}

/// Returns `true` when the database error info points to the
/// one-active-per-conversation partial unique index.
fn is_active_session_constraint(info: &dyn DatabaseErrorInformation) -> bool {
    info.constraint_name()
        .is_some_and(|name| name == ACTIVE_SESSION_UNIQUE_INDEX)
}

/// Maps a `UniqueViolation` on insert to either `Duplicate` (PK clash) or
/// `ActiveSessionExists` (partial unique index clash), falling back to a
/// generic persistence error for anything else.
pub(super) fn map_insert_error(
    err: DieselError,
    session_id: AgentSessionId,
    conversation_id: ConversationId,
) -> SessionError {
    if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, ref info) = err {
        if is_active_session_constraint(info.as_ref()) {
            return SessionError::ActiveSessionExists(conversation_id);
        }
        return SessionError::Duplicate(session_id);
    }
    SessionError::persistence(err)
}

/// Maps a `UniqueViolation` on update to `ActiveSessionExists` (partial unique
/// index clash), falling back to a generic persistence error for anything else.
pub(super) fn map_update_error(
    err: DieselError,
    _session_id: AgentSessionId,
    conversation_id: ConversationId,
) -> SessionError {
    if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, ref info) = err
        && is_active_session_constraint(info.as_ref())
    {
        return SessionError::ActiveSessionExists(conversation_id);
    }
    SessionError::persistence(err)
}

/// Checks that no other active session exists for the given conversation.
///
/// When `exclude_id` is `Some`, the check ignores the session with that ID
/// (used during updates to allow re-saving the same active session).
pub(super) fn check_no_active_session(
    conn: &mut PgConnection,
    conversation_id: ConversationId,
    exclude_id: Option<AgentSessionId>,
) -> SessionResult<()> {
    let active_state = AgentSessionState::Active.as_str();
    let conv_uuid = conversation_id.into_inner();

    let mut query = agent_sessions::table
        .filter(agent_sessions::conversation_id.eq(conv_uuid))
        .filter(agent_sessions::state.eq(active_state))
        .into_boxed();

    if let Some(id) = exclude_id {
        query = query.filter(agent_sessions::id.ne(id.into_inner()));
    }

    let count: i64 = query
        .count()
        .get_result(conn)
        .map_err(SessionError::persistence)?;

    if count > 0 {
        return Err(SessionError::ActiveSessionExists(conversation_id));
    }

    Ok(())
}
