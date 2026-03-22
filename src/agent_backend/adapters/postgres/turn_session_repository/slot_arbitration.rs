//! Slot arbitration helpers for `PostgreSQL` turn-session persistence.

use crate::agent_backend::{
    domain::{BackendId, TurnSession, TurnSessionStatus},
    ports::{
        SessionSlotArbitration, SessionSlotReservation, TurnSessionRepositoryError,
        TurnSessionRepositoryResult,
    },
};
use chrono::{DateTime, Duration, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;

use super::super::{models::AgentTurnSessionRow, schema::agent_turn_sessions};
use super::{
    map_upsert_error,
    row_mapping::{create_reservation_session, row_to_turn_session, to_new_row},
};

pub(crate) fn arbitrate_session_slot_tx(
    tx_conn: &mut PgConnection,
    tenant_id: uuid::Uuid,
    slot_reservation: SessionSlotReservation,
) -> TurnSessionRepositoryResult<SessionSlotArbitration> {
    let SessionSlotReservation { key, now, ttl } = slot_reservation;
    let backend_id = key.backend_id;
    let conversation_id = key.conversation_id;

    lock_session_key(tx_conn, tenant_id, backend_id.into_inner(), conversation_id)?;

    let row = agent_turn_sessions::table
        .filter(agent_turn_sessions::tenant_id.eq(tenant_id))
        .filter(agent_turn_sessions::backend_id.eq(backend_id.into_inner()))
        .filter(agent_turn_sessions::conversation_id.eq(conversation_id))
        .filter(agent_turn_sessions::status.eq(TurnSessionStatus::Active.as_str()))
        .order(agent_turn_sessions::last_used_at.desc())
        .for_update()
        .select(AgentTurnSessionRow::as_select())
        .first::<AgentTurnSessionRow>(tx_conn)
        .optional()
        .map_err(TurnSessionRepositoryError::persistence)?;

    let Some(existing_row) = row else {
        let reservation = insert_reserved_session(
            tx_conn,
            ReservedSessionInsertParams {
                backend_id,
                conversation_id,
                tenant_id,
                now,
                ttl,
            },
        )?;
        return Ok(SessionSlotArbitration::Reserved {
            reservation,
            prior_expired: None,
        });
    };

    let existing = row_to_turn_session(existing_row)?;
    if existing.is_expired_at(now) {
        let prior_expired = expire_active_session(tx_conn, &existing, now)?;
        let reservation = insert_reserved_session(
            tx_conn,
            ReservedSessionInsertParams {
                backend_id,
                conversation_id,
                tenant_id,
                now,
                ttl,
            },
        )?;
        return Ok(SessionSlotArbitration::Reserved {
            reservation,
            prior_expired: Some(prior_expired),
        });
    }

    Ok(SessionSlotArbitration::Reused(existing))
}

// Acquires a row-level lock on the conversation-scoped session row when one is
// already present for the tenant/backend/conversation slot.
//
// Vacant slots intentionally fall through without locking; concurrent vacancy
// observations are serialized by the partial unique index that now covers both
// active and reserved slot claims.
pub(crate) fn lock_session_key(
    connection: &mut PgConnection,
    tenant_id: uuid::Uuid,
    backend_id: uuid::Uuid,
    conversation_id: uuid::Uuid,
) -> TurnSessionRepositoryResult<()> {
    agent_turn_sessions::table
        .filter(agent_turn_sessions::tenant_id.eq(tenant_id))
        .filter(agent_turn_sessions::backend_id.eq(backend_id))
        .filter(agent_turn_sessions::conversation_id.eq(conversation_id))
        .for_update()
        .select(agent_turn_sessions::id)
        .first::<uuid::Uuid>(connection)
        .optional()
        .map(|_| ())
        .map_err(TurnSessionRepositoryError::persistence)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReservedSessionInsertParams {
    backend_id: BackendId,
    conversation_id: uuid::Uuid,
    tenant_id: uuid::Uuid,
    now: DateTime<Utc>,
    ttl: Duration,
}

/// Inserts a new reserved-session row and returns the domain session.
pub(crate) fn insert_reserved_session(
    conn: &mut PgConnection,
    params: ReservedSessionInsertParams,
) -> TurnSessionRepositoryResult<TurnSession> {
    let reserved = create_reservation_session(
        params.backend_id,
        params.conversation_id,
        params.now,
        params.ttl,
    )?;
    diesel::insert_into(agent_turn_sessions::table)
        .values(to_new_row(&reserved, params.tenant_id)?)
        .execute(conn)
        .map_err(|error| {
            map_upsert_error(
                error,
                params.backend_id.into_inner(),
                params.conversation_id,
            )
        })?;
    Ok(reserved)
}

/// Updates an active session to `Expired` in the database and returns the
/// updated domain session.
pub(crate) fn expire_active_session(
    conn: &mut PgConnection,
    existing: &TurnSession,
    now: DateTime<Utc>,
) -> TurnSessionRepositoryResult<TurnSession> {
    diesel::update(
        agent_turn_sessions::table.filter(agent_turn_sessions::id.eq(existing.id().into_inner())),
    )
    .set((
        agent_turn_sessions::status.eq(TurnSessionStatus::Expired.as_str()),
        agent_turn_sessions::ended_at.eq(Some(now)),
    ))
    .execute(conn)
    .map_err(TurnSessionRepositoryError::persistence)?;
    let mut expired = existing.clone();
    expired.mark_expired(now);
    Ok(expired)
}
