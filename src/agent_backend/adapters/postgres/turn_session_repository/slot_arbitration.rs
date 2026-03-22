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
use diesel::sql_types::{Uuid as SqlUuid, Varchar};

use super::super::{
    models::AgentTurnSessionRow,
    schema::{agent_turn_sessions, backend_registrations},
};
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
    let params = ReservedSessionInsertParams {
        backend_id: key.backend_id,
        conversation_id: key.conversation_id,
        tenant_id,
        now,
        ttl,
    };

    lock_session_key(
        tx_conn,
        tenant_id,
        params.backend_id.into_inner(),
        params.conversation_id,
    )?;
    expire_stale_reservations(tx_conn, params)?;

    match load_claimed_session(tx_conn, params)? {
        Some(existing_row) => {
            handle_existing_claim(tx_conn, params, row_to_turn_session(existing_row)?)
        }
        None => reserve_slot(tx_conn, params, None),
    }
}

// Acquires a row-level lock on the owning backend registration so empty-slot
// arbitration is serialized by a durable database sentinel row.
pub(crate) fn lock_session_key(
    connection: &mut PgConnection,
    tenant_id: uuid::Uuid,
    backend_id: uuid::Uuid,
    _conversation_id: uuid::Uuid,
) -> TurnSessionRepositoryResult<()> {
    backend_registrations::table
        .filter(backend_registrations::tenant_id.eq(tenant_id))
        .filter(backend_registrations::id.eq(backend_id))
        .for_update()
        .select(backend_registrations::id)
        .first::<uuid::Uuid>(connection)
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
    let inserted = diesel::insert_into(agent_turn_sessions::table)
        .values(to_new_row(&reserved, params.tenant_id)?)
        .on_conflict((
            agent_turn_sessions::tenant_id,
            agent_turn_sessions::backend_id,
            agent_turn_sessions::conversation_id,
        ))
        .filter_target(agent_turn_sessions::status.eq_any([
            TurnSessionStatus::Active.as_str(),
            TurnSessionStatus::Reserved.as_str(),
        ]))
        .do_nothing()
        .execute(conn)
        .map_err(|error| {
            map_upsert_error(
                error,
                params.backend_id.into_inner(),
                params.conversation_id,
            )
        })?;
    if inserted == 0 {
        return Err(TurnSessionRepositoryError::active_session_conflict(
            params.backend_id,
            params.conversation_id,
        ));
    }
    Ok(reserved)
}

/// Updates a claimed session to `Expired` in the database and returns the
/// updated domain session.
pub(crate) fn expire_claimed_session(
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

fn load_claimed_session(
    tx_conn: &mut PgConnection,
    params: ReservedSessionInsertParams,
) -> TurnSessionRepositoryResult<Option<AgentTurnSessionRow>> {
    diesel::sql_query(
        "SELECT id, tenant_id, backend_id, conversation_id, runtime_session_id, status, \
         ttl_seconds, started_at, last_used_at, expires_at, ended_at, turn_count \
         FROM agent_turn_sessions \
         WHERE tenant_id = $1 AND backend_id = $2 AND conversation_id = $3 \
           AND status IN ($4, $5) \
         ORDER BY last_used_at DESC, id DESC \
         FOR UPDATE",
    )
    .bind::<SqlUuid, _>(params.tenant_id)
    .bind::<SqlUuid, _>(params.backend_id.into_inner())
    .bind::<SqlUuid, _>(params.conversation_id)
    .bind::<Varchar, _>(TurnSessionStatus::Active.as_str())
    .bind::<Varchar, _>(TurnSessionStatus::Reserved.as_str())
    .get_result::<AgentTurnSessionRow>(tx_conn)
    .optional()
    .map_err(TurnSessionRepositoryError::persistence)
}

fn handle_existing_claim(
    tx_conn: &mut PgConnection,
    params: ReservedSessionInsertParams,
    existing: TurnSession,
) -> TurnSessionRepositoryResult<SessionSlotArbitration> {
    match existing.status() {
        TurnSessionStatus::Active if is_active_and_expired(&existing, params.now) => {
            let prior_expired = expire_claimed_session(tx_conn, &existing, params.now)?;
            reserve_slot(tx_conn, params, Some(prior_expired))
        }
        TurnSessionStatus::Active => Ok(SessionSlotArbitration::Reused(existing)),
        TurnSessionStatus::Reserved if is_reserved_and_expired(&existing, params.now) => {
            expire_claimed_session(tx_conn, &existing, params.now)?;
            reserve_slot(tx_conn, params, None)
        }
        TurnSessionStatus::Reserved => Err(TurnSessionRepositoryError::active_session_conflict(
            params.backend_id,
            params.conversation_id,
        )),
        TurnSessionStatus::Expired => Err(TurnSessionRepositoryError::invalid_persisted_data(
            std::io::Error::other("expired session row should not claim a session slot"),
        )),
    }
}

fn is_active_and_expired(session: &TurnSession, now: DateTime<Utc>) -> bool {
    session.status() == TurnSessionStatus::Active && session.is_expired_at(now)
}

fn is_reserved_and_expired(session: &TurnSession, now: DateTime<Utc>) -> bool {
    session.status() == TurnSessionStatus::Reserved && session.is_expired_at(now)
}

fn reserve_slot(
    tx_conn: &mut PgConnection,
    params: ReservedSessionInsertParams,
    prior_expired: Option<TurnSession>,
) -> TurnSessionRepositoryResult<SessionSlotArbitration> {
    let reservation = insert_reserved_session(tx_conn, params)?;
    Ok(SessionSlotArbitration::Reserved {
        reservation,
        prior_expired,
    })
}

fn expire_stale_reservations(
    conn: &mut PgConnection,
    params: ReservedSessionInsertParams,
) -> TurnSessionRepositoryResult<()> {
    diesel::update(
        agent_turn_sessions::table
            .filter(agent_turn_sessions::tenant_id.eq(params.tenant_id))
            .filter(agent_turn_sessions::backend_id.eq(params.backend_id.into_inner()))
            .filter(agent_turn_sessions::conversation_id.eq(params.conversation_id))
            .filter(agent_turn_sessions::status.eq(TurnSessionStatus::Reserved.as_str()))
            .filter(agent_turn_sessions::expires_at.le(params.now)),
    )
    .set((
        agent_turn_sessions::status.eq(TurnSessionStatus::Expired.as_str()),
        agent_turn_sessions::ended_at.eq(Some(params.now)),
    ))
    .execute(conn)
    .map(|_| ())
    .map_err(TurnSessionRepositoryError::persistence)
}
