//! Row mapping helpers for turn-session persistence.

use crate::agent_backend::domain::{
    BackendId, PersistedTurnSessionData, ReservedTurnSessionCreateParams, RuntimeSessionId,
    TurnSession, TurnSessionId, TurnSessionStatus,
};
use crate::agent_backend::ports::{TurnSessionRepositoryError, TurnSessionRepositoryResult};
use chrono::{DateTime, Duration, Utc};

use super::super::models::{AgentTurnSessionRow, NewAgentTurnSessionRow};

pub(crate) fn to_new_row(
    session: &TurnSession,
    tenant_id: uuid::Uuid,
) -> TurnSessionRepositoryResult<NewAgentTurnSessionRow> {
    let turn_count: i64 = session
        .turn_count()
        .try_into()
        .map_err(TurnSessionRepositoryError::invalid_domain_data)?;

    Ok(NewAgentTurnSessionRow {
        id: session.id().into_inner(),
        tenant_id,
        backend_id: session.backend_id().into_inner(),
        conversation_id: session.conversation_id(),
        runtime_session_id: session.runtime_session_handle().as_str().to_owned(),
        status: session.status().as_str().to_owned(),
        ttl_seconds: session.ttl_seconds(),
        started_at: session.started_at(),
        last_used_at: session.last_used_at(),
        expires_at: session.expires_at(),
        ended_at: session.ended_at(),
        turn_count,
    })
}

pub(crate) fn create_reservation_session(
    backend_id: BackendId,
    conversation_id: uuid::Uuid,
    now: DateTime<Utc>,
    ttl: Duration,
) -> TurnSessionRepositoryResult<TurnSession> {
    TurnSession::new_reserved(&ReservedTurnSessionCreateParams {
        id: TurnSessionId::new(),
        backend_id,
        conversation_id,
        ttl,
        now,
    })
    .map_err(TurnSessionRepositoryError::invalid_domain_data)
}

pub(crate) fn row_to_turn_session(
    row: AgentTurnSessionRow,
) -> TurnSessionRepositoryResult<TurnSession> {
    let AgentTurnSessionRow {
        id,
        tenant_id: _tenant_id,
        backend_id,
        conversation_id,
        runtime_session_id,
        status,
        ttl_seconds,
        started_at,
        last_used_at,
        expires_at,
        ended_at,
        turn_count,
    } = row;

    let parsed_status = TurnSessionStatus::try_from(status.as_str())
        .map_err(TurnSessionRepositoryError::invalid_persisted_data)?;
    let parsed_runtime_session_id = RuntimeSessionId::new(runtime_session_id)
        .map_err(TurnSessionRepositoryError::invalid_persisted_data)?;
    let parsed_turn_count: u64 =
        turn_count
            .try_into()
            .map_err(|err: std::num::TryFromIntError| {
                TurnSessionRepositoryError::invalid_persisted_data(err)
            })?;

    Ok(TurnSession::from_persisted(PersistedTurnSessionData {
        id: TurnSessionId::from_uuid(id),
        backend_id: BackendId::from_uuid(backend_id),
        conversation_id,
        runtime_session_id: parsed_runtime_session_id,
        status: parsed_status,
        ttl_seconds,
        started_at,
        last_used_at,
        expires_at,
        ended_at,
        turn_count: parsed_turn_count,
    }))
}
