//! `PostgreSQL` repository implementation for orchestration turn sessions.

use super::{
    models::{AgentTurnSessionRow, NewAgentTurnSessionRow},
    repository::BackendPgPool,
    schema::agent_turn_sessions,
};
use crate::agent_backend::{
    domain::{BackendId, PersistedTurnSessionData, TurnSession, TurnSessionId, TurnSessionStatus},
    ports::{TurnSessionRepository, TurnSessionRepositoryError, TurnSessionRepositoryResult},
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;

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
    async fn find_active_session(
        &self,
        backend_id: BackendId,
        conversation_id: uuid::Uuid,
    ) -> TurnSessionRepositoryResult<Option<TurnSession>> {
        self.run_blocking(move |connection| {
            let row = agent_turn_sessions::table
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

    async fn upsert_session(&self, session: &TurnSession) -> TurnSessionRepositoryResult<()> {
        let new_row = to_new_row(session)?;

        self.run_blocking(move |connection| {
            diesel::insert_into(agent_turn_sessions::table)
                .values(&new_row)
                .on_conflict(agent_turn_sessions::id)
                .do_update()
                .set((
                    agent_turn_sessions::status.eq(&new_row.status),
                    agent_turn_sessions::runtime_session_id.eq(&new_row.runtime_session_id),
                    agent_turn_sessions::ttl_seconds.eq(new_row.ttl_seconds),
                    agent_turn_sessions::last_used_at.eq(new_row.last_used_at),
                    agent_turn_sessions::expires_at.eq(new_row.expires_at),
                    agent_turn_sessions::ended_at.eq(new_row.ended_at),
                    agent_turn_sessions::turn_count.eq(new_row.turn_count),
                ))
                .execute(connection)
                .map_err(TurnSessionRepositoryError::persistence)?;
            Ok(())
        })
        .await
    }
}

fn to_new_row(session: &TurnSession) -> TurnSessionRepositoryResult<NewAgentTurnSessionRow> {
    let turn_count: i64 =
        session
            .turn_count()
            .try_into()
            .map_err(|err: std::num::TryFromIntError| {
                TurnSessionRepositoryError::invalid_persisted_data(std::io::Error::other(
                    err.to_string(),
                ))
            })?;

    Ok(NewAgentTurnSessionRow {
        id: session.id().into_inner(),
        backend_id: session.backend_id().into_inner(),
        conversation_id: session.conversation_id(),
        runtime_session_id: session.runtime_session_id().to_owned(),
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
        runtime_session_id,
        status: parsed_status,
        ttl_seconds,
        started_at,
        last_used_at,
        expires_at,
        ended_at,
        turn_count: parsed_turn_count,
    }))
}
