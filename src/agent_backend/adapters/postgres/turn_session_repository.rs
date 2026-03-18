//! `PostgreSQL` repository implementation for orchestration turn sessions.

use super::{
    models::{AgentTurnSessionRow, NewAgentTurnSessionRow},
    repository::BackendPgPool,
    schema::{agent_turn_sessions, backend_registrations},
};
use crate::agent_backend::{
    domain::{
        BackendId, PersistedTurnSessionData, RuntimeSessionId, TurnSession, TurnSessionId,
        TurnSessionStatus,
    },
    ports::{
        SessionSlotArbitration, SessionSlotKey, TurnSessionRepository, TurnSessionRepositoryError,
        TurnSessionRepositoryResult,
    },
};
use crate::context::RequestContext;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

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
}

#[async_trait]
impl TurnSessionRepository for PostgresTurnSessionRepository {
    async fn arbitrate_session_slot(
        &self,
        ctx: &RequestContext,
        key: SessionSlotKey,
        now: DateTime<Utc>,
    ) -> TurnSessionRepositoryResult<SessionSlotArbitration> {
        let SessionSlotKey { backend_id, conversation_id } = key;
        let tenant_id = ctx.tenant_id().into_inner();
        self.run_blocking(move |connection| {
            connection.transaction(|tx_conn| {
                lock_session_key(tx_conn, tenant_id, backend_id.into_inner())?;

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
                    return Ok(SessionSlotArbitration::Vacant);
                };

                let existing = row_to_turn_session(existing_row)?;
                if existing.is_expired_at(now) {
                    diesel::update(
                        agent_turn_sessions::table
                            .filter(agent_turn_sessions::id.eq(existing.id().into_inner())),
                    )
                    .set((
                        agent_turn_sessions::status.eq(TurnSessionStatus::Expired.as_str()),
                        agent_turn_sessions::ended_at.eq(Some(now)),
                    ))
                    .execute(tx_conn)
                    .map_err(TurnSessionRepositoryError::persistence)?;
                    return Ok(SessionSlotArbitration::Expired);
                }

                Ok(SessionSlotArbitration::Reused(existing))
            })
        })
        .await
    }

    async fn find_active_session(
        &self,
        ctx: &RequestContext,
        key: SessionSlotKey,
    ) -> TurnSessionRepositoryResult<Option<TurnSession>> {
        let SessionSlotKey { backend_id, conversation_id } = key;
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
                lock_session_key(tx_conn, tenant_id, backend_id)?;

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
                    .filter_target(
                        agent_turn_sessions::status.eq(TurnSessionStatus::Active.as_str()),
                    )
                    .do_nothing()
                    .execute(tx_conn)
                    .map_err(|error| map_upsert_error(error, backend_id, conversation_id))?;

                if inserted == 0 && new_row.status == TurnSessionStatus::Active.as_str() {
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

// Acquires a row-level lock on the backend_registrations row to serialize
// concurrent turn executions for the same (backend_id, conversation_id) pair.
// We lock the backend_registrations row instead of using pg_advisory_xact_lock
// because: (1) it avoids hash collisions across different key spaces, (2) it
// automatically verifies the backend registration exists (fail-fast if not
// found), and (3) it leverages the database's row-level locking semantics to
// keep locking and existence checks atomic.
fn lock_session_key(
    connection: &mut PgConnection,
    tenant_id: uuid::Uuid,
    backend_id: uuid::Uuid,
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

fn to_new_row(
    session: &TurnSession,
    tenant_id: uuid::Uuid,
) -> TurnSessionRepositoryResult<NewAgentTurnSessionRow> {
    let turn_count: i64 =
        session
            .turn_count()
            .try_into()
            .map_err(|err: std::num::TryFromIntError| {
                TurnSessionRepositoryError::invalid_domain_data(std::io::Error::other(
                    err.to_string(),
                ))
            })?;

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

fn row_to_turn_session(row: AgentTurnSessionRow) -> TurnSessionRepositoryResult<TurnSession> {
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
                TurnSessionRepositoryError::invalid_persisted_data(std::io::Error::other(
                    err.to_string(),
                ))
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

fn map_upsert_error(
    error: DieselError,
    backend_id: uuid::Uuid,
    conversation_id: uuid::Uuid,
) -> TurnSessionRepositoryError {
    let is_active_session_conflict = matches!(
        &error,
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, db_error)
            if db_error
                .constraint_name()
                .is_some_and(|name| name == "idx_agent_turn_sessions_backend_conversation_active")
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
