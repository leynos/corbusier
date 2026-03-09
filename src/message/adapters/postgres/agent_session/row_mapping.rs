//! Row-to-domain and domain-to-row mapping for agent sessions.
//!
//! Converts between Diesel row types ([`AgentSessionRow`], [`NewAgentSession`])
//! and the domain [`AgentSession`] aggregate, handling JSONB serialization of
//! turn IDs and context snapshots.

use diesel::prelude::*;
use serde::Serialize;

use crate::message::{
    adapters::models::{AgentSessionRow, NewAgentSession},
    adapters::schema::agent_sessions,
    domain::{
        AgentSession, AgentSessionId, AgentSessionState, ContextWindowSnapshot, ConversationId,
        HandoffId, SequenceNumber, TurnId,
    },
    ports::agent_session::{SessionError, SessionResult},
};

/// Changeset for updating an agent session.
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = agent_sessions)]
pub(super) struct AgentSessionUpdate {
    pub end_sequence: Option<i64>,
    pub turn_ids: serde_json::Value,
    pub terminated_by_handoff: Option<uuid::Uuid>,
    pub context_snapshots: serde_json::Value,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub state: String,
}

/// Converts a domain `AgentSession` to a `NewAgentSession` for insertion.
pub(super) fn session_to_new_row(session: &AgentSession) -> SessionResult<NewAgentSession> {
    let turn_ids = serialize_json(&session.turn_ids)?;

    let context_snapshots = serialize_json(&session.context_snapshots)?;

    let start_sequence =
        i64::try_from(session.start_sequence.value()).map_err(SessionError::persistence)?;

    let end_sequence = session_end_sequence(session)?;

    Ok(NewAgentSession {
        id: session.session_id.into_inner(),
        conversation_id: session.conversation_id.into_inner(),
        agent_backend: session.agent_backend.clone(),
        start_sequence,
        end_sequence,
        turn_ids,
        initiated_by_handoff: session.initiated_by_handoff.map(HandoffId::into_inner),
        terminated_by_handoff: session.terminated_by_handoff.map(HandoffId::into_inner),
        context_snapshots,
        started_at: session.started_at,
        ended_at: session.ended_at,
        state: session.state.as_str().to_owned(),
    })
}

/// Converts a domain `AgentSession` to update values.
pub(super) fn session_to_update_values(
    session: &AgentSession,
) -> SessionResult<AgentSessionUpdate> {
    let turn_ids = serialize_json(&session.turn_ids)?;

    let context_snapshots = serialize_json(&session.context_snapshots)?;

    let end_sequence = session_end_sequence(session)?;

    Ok(AgentSessionUpdate {
        end_sequence,
        turn_ids,
        terminated_by_handoff: session.terminated_by_handoff.map(HandoffId::into_inner),
        context_snapshots,
        ended_at: session.ended_at,
        state: session.state.as_str().to_owned(),
    })
}

/// Converts a database row to a domain `AgentSession`.
pub(super) fn row_to_session(row: AgentSessionRow) -> SessionResult<AgentSession> {
    let turn_ids: Vec<TurnId> =
        serde_json::from_value(row.turn_ids).map_err(SessionError::persistence)?;

    let context_snapshots: Vec<ContextWindowSnapshot> =
        serde_json::from_value(row.context_snapshots).map_err(SessionError::persistence)?;

    let start_sequence = u64::try_from(row.start_sequence).map_err(SessionError::persistence)?;

    let end_sequence = row
        .end_sequence
        .map(u64::try_from)
        .transpose()
        .map_err(SessionError::persistence)?
        .map(SequenceNumber::new);

    let state =
        AgentSessionState::try_from(row.state.as_str()).map_err(SessionError::persistence)?;

    Ok(AgentSession {
        session_id: AgentSessionId::from_uuid(row.id),
        conversation_id: ConversationId::from_uuid(row.conversation_id),
        agent_backend: row.agent_backend,
        start_sequence: SequenceNumber::new(start_sequence),
        end_sequence,
        turn_ids,
        initiated_by_handoff: row.initiated_by_handoff.map(HandoffId::from_uuid),
        terminated_by_handoff: row.terminated_by_handoff.map(HandoffId::from_uuid),
        context_snapshots,
        started_at: row.started_at,
        ended_at: row.ended_at,
        state,
    })
}

fn serialize_json<T: Serialize>(value: &T) -> SessionResult<serde_json::Value> {
    serde_json::to_value(value).map_err(SessionError::persistence)
}

fn session_end_sequence(session: &AgentSession) -> SessionResult<Option<i64>> {
    session
        .end_sequence
        .map(|s| i64::try_from(s.value()))
        .transpose()
        .map_err(SessionError::persistence)
}
