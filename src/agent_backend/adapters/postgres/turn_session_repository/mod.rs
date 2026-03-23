//! `PostgreSQL` repository implementation for orchestration turn sessions.

mod row_mapping;
mod slot_arbitration;

use super::{models::AgentTurnSessionRow, repository::BackendPgPool, schema::agent_turn_sessions};
use crate::agent_backend::{
    domain::{BackendId, TurnSession, TurnSessionStatus},
    ports::{
        SessionSlotArbitration, SessionSlotKey, SessionSlotReservation, TurnSessionRepository,
        TurnSessionRepositoryError, TurnSessionRepositoryResult,
    },
};
use crate::context::RequestContext;
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use row_mapping::{row_to_turn_session, to_new_row};
use slot_arbitration::arbitrate_session_slot_tx;

pub(crate) const CONSTRAINT_IDX_TURN_SESSIONS_ACTIVE: &str =
    "idx_agent_turn_sessions_tenant_backend_conversation_active";

/// `PostgreSQL`-backed turn-session repository.
#[derive(Debug, Clone)]
pub struct PostgresTurnSessionRepository {
    pool: BackendPgPool,
}

impl PostgresTurnSessionRepository {
    /// Creates a new repository from a `PostgreSQL` connection pool.
    #[must_use]
    pub const fn new(pool: BackendPgPool) -> Self {
        Self { pool }
    }

    async fn run_blocking<F, T>(&self, f: F) -> TurnSessionRepositoryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> TurnSessionRepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool
                .get()
                .map_err(TurnSessionRepositoryError::persistence)?;
            f(&mut connection)
        })
        .await
        .map_err(TurnSessionRepositoryError::persistence)?
    }

    /// Returns all persisted sessions for integration-test assertions.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionRepositoryError`] on persistence failures.
    #[cfg(any(test, feature = "test-support"))]
    pub fn all_sessions(&self) -> TurnSessionRepositoryResult<Vec<TurnSession>> {
        let mut connection = self
            .pool
            .get()
            .map_err(TurnSessionRepositoryError::persistence)?;
        let rows = agent_turn_sessions::table
            .order(agent_turn_sessions::started_at.asc())
            .select(AgentTurnSessionRow::as_select())
            .load::<AgentTurnSessionRow>(&mut connection)
            .map_err(TurnSessionRepositoryError::persistence)?;

        rows.into_iter().map(row_to_turn_session).collect()
    }
}

#[async_trait]
impl TurnSessionRepository for PostgresTurnSessionRepository {
    async fn arbitrate_session_slot(
        &self,
        ctx: &RequestContext,
        slot_reservation: SessionSlotReservation,
    ) -> TurnSessionRepositoryResult<SessionSlotArbitration> {
        let tenant_id = ctx.tenant_id().into_inner();
        self.run_blocking(move |connection| {
            connection.transaction(|tx_conn| {
                arbitrate_session_slot_tx(tx_conn, tenant_id, slot_reservation)
            })
        })
        .await
    }

    async fn find_active_session(
        &self,
        ctx: &RequestContext,
        key: SessionSlotKey,
    ) -> TurnSessionRepositoryResult<Option<TurnSession>> {
        let SessionSlotKey {
            backend_id,
            conversation_id,
        } = key;
        let tenant_id = ctx.tenant_id().into_inner();
        self.run_blocking(move |connection| {
            let row = agent_turn_sessions::table
                .filter(agent_turn_sessions::tenant_id.eq(tenant_id))
                .filter(agent_turn_sessions::backend_id.eq(backend_id.into_inner()))
                .filter(agent_turn_sessions::conversation_id.eq(conversation_id))
                .filter(agent_turn_sessions::status.eq(TurnSessionStatus::Active.as_str()))
                .order(agent_turn_sessions::last_used_at.desc())
                .select(AgentTurnSessionRow::as_select())
                .first::<AgentTurnSessionRow>(connection)
                .optional()
                .map_err(TurnSessionRepositoryError::persistence)?;

            row.map(row_to_turn_session).transpose()
        })
        .await
    }

    async fn upsert_session(
        &self,
        ctx: &RequestContext,
        session: &TurnSession,
    ) -> TurnSessionRepositoryResult<()> {
        let tenant_id = ctx.tenant_id().into_inner();
        let new_row = to_new_row(session, tenant_id)?;
        let backend_id = new_row.backend_id;
        let conversation_id = new_row.conversation_id;

        self.run_blocking(move |connection| {
            connection.transaction(|tx_conn| {
                slot_arbitration::lock_session_key(tx_conn, tenant_id, backend_id)?;

                let updated = diesel::update(
                    agent_turn_sessions::table
                        .filter(agent_turn_sessions::id.eq(new_row.id))
                        .filter(agent_turn_sessions::tenant_id.eq(tenant_id)),
                )
                .set((
                    agent_turn_sessions::status.eq(&new_row.status),
                    agent_turn_sessions::runtime_session_id.eq(&new_row.runtime_session_id),
                    agent_turn_sessions::ttl_seconds.eq(new_row.ttl_seconds),
                    agent_turn_sessions::started_at.eq(new_row.started_at),
                    agent_turn_sessions::last_used_at.eq(new_row.last_used_at),
                    agent_turn_sessions::expires_at.eq(new_row.expires_at),
                    agent_turn_sessions::ended_at.eq(new_row.ended_at),
                    agent_turn_sessions::turn_count.eq(new_row.turn_count),
                ))
                .execute(tx_conn)
                .map_err(TurnSessionRepositoryError::persistence)?;

                if updated > 0 {
                    return Ok(());
                }

                let inserted = diesel::insert_into(agent_turn_sessions::table)
                    .values(&new_row)
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
                    .execute(tx_conn)
                    .map_err(|error| map_upsert_error(error, backend_id, conversation_id))?;

                if inserted == 0 && is_slot_claiming_status(new_row.status.as_str()) {
                    return Err(TurnSessionRepositoryError::active_session_conflict(
                        BackendId::from_uuid(backend_id),
                        conversation_id,
                    ));
                }
                Ok(())
            })
        })
        .await
    }
}

/// Returns `true` when `status` represents a session that claims an active
/// slot — i.e. `active` or `reserved`.
fn is_slot_claiming_status(status: &str) -> bool {
    status == TurnSessionStatus::Active.as_str() || status == TurnSessionStatus::Reserved.as_str()
}

pub(crate) fn map_upsert_error(
    error: DieselError,
    backend_id: uuid::Uuid,
    conversation_id: uuid::Uuid,
) -> TurnSessionRepositoryError {
    let is_active_session_conflict = matches!(
        &error,
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, db_error)
            if db_error
                .constraint_name()
                .is_some_and(|name| name == CONSTRAINT_IDX_TURN_SESSIONS_ACTIVE)
    );
    if is_active_session_conflict {
        TurnSessionRepositoryError::active_session_conflict(
            BackendId::from_uuid(backend_id),
            conversation_id,
        )
    } else {
        TurnSessionRepositoryError::persistence(error)
    }
}

impl From<DieselError> for TurnSessionRepositoryError {
    fn from(error: DieselError) -> Self {
        Self::persistence(error)
    }
}
