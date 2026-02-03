//! `PostgreSQL` implementation of the `AgentSessionRepository` port using Diesel ORM.
//!
//! Provides production-grade persistence for agent sessions with JSONB storage
//! for turn IDs and context snapshots.

use async_trait::async_trait;
use diesel::prelude::*;

use super::blocking_helpers::PgPool;
use crate::message::{
    adapters::models::{AgentSessionRow, NewAgentSession},
    adapters::schema::agent_sessions,
    domain::{
        AgentSession, AgentSessionId, AgentSessionState, ContextWindowSnapshot, ConversationId,
        HandoffId, SequenceNumber, TurnId,
    },
    ports::agent_session::{AgentSessionRepository, SessionError, SessionResult},
};

/// `PostgreSQL` implementation of [`AgentSessionRepository`].
///
/// Uses Diesel ORM with connection pooling via r2d2. Thread-safe for
/// concurrent access.
#[derive(Debug, Clone)]
pub struct PostgresAgentSessionRepository {
    pool: PgPool,
}

impl PostgresAgentSessionRepository {
    /// Creates a new repository with the given connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AgentSessionRepository for PostgresAgentSessionRepository {
    async fn store(&self, session: &AgentSession) -> SessionResult<()> {
        let pool = self.pool.clone();
        let new_session = session_to_new_row(session)?;
        let session_id = session.session_id;

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            // Check for duplicate
            let exists: i64 = agent_sessions::table
                .filter(agent_sessions::id.eq(session_id.into_inner()))
                .count()
                .get_result(&mut conn)
                .map_err(SessionError::persistence)?;

            if exists > 0 {
                return Err(SessionError::Duplicate(session_id));
            }

            diesel::insert_into(agent_sessions::table)
                .values(&new_session)
                .execute(&mut conn)
                .map_err(SessionError::persistence)?;

            Ok(())
        })
        .await
    }

    async fn update(&self, session: &AgentSession) -> SessionResult<()> {
        let pool = self.pool.clone();
        let session_id = session.session_id;
        let updated = session_to_update_values(session)?;

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            let updated_rows = diesel::update(
                agent_sessions::table.filter(agent_sessions::id.eq(session_id.into_inner())),
            )
            .set(&updated)
            .execute(&mut conn)
            .map_err(SessionError::persistence)?;

            if updated_rows == 0 {
                return Err(SessionError::NotFound(session_id));
            }

            Ok(())
        })
        .await
    }

    async fn find_by_id(&self, id: AgentSessionId) -> SessionResult<Option<AgentSession>> {
        let pool = self.pool.clone();
        let uuid = id.into_inner();

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            agent_sessions::table
                .filter(agent_sessions::id.eq(uuid))
                .select(AgentSessionRow::as_select())
                .first::<AgentSessionRow>(&mut conn)
                .optional()
                .map_err(SessionError::persistence)?
                .map(row_to_session)
                .transpose()
        })
        .await
    }

    async fn find_active_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> SessionResult<Option<AgentSession>> {
        let pool = self.pool.clone();
        let uuid = conversation_id.into_inner();

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            agent_sessions::table
                .filter(agent_sessions::conversation_id.eq(uuid))
                .filter(agent_sessions::state.eq("active"))
                .select(AgentSessionRow::as_select())
                .first::<AgentSessionRow>(&mut conn)
                .optional()
                .map_err(SessionError::persistence)?
                .map(row_to_session)
                .transpose()
        })
        .await
    }

    async fn find_by_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> SessionResult<Vec<AgentSession>> {
        let pool = self.pool.clone();
        let uuid = conversation_id.into_inner();

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            let rows = agent_sessions::table
                .filter(agent_sessions::conversation_id.eq(uuid))
                .order(agent_sessions::started_at.asc())
                .select(AgentSessionRow::as_select())
                .load::<AgentSessionRow>(&mut conn)
                .map_err(SessionError::persistence)?;

            rows.into_iter().map(row_to_session).collect()
        })
        .await
    }
}

/// Converts a domain `AgentSession` to a `NewAgentSession` for insertion.
fn session_to_new_row(session: &AgentSession) -> SessionResult<NewAgentSession> {
    let turn_ids = serde_json::to_value(&session.turn_ids).map_err(SessionError::persistence)?;

    let context_snapshots =
        serde_json::to_value(&session.context_snapshots).map_err(SessionError::persistence)?;

    let start_sequence =
        i64::try_from(session.start_sequence.value()).map_err(SessionError::persistence)?;

    let end_sequence = session
        .end_sequence
        .map(|s| i64::try_from(s.value()))
        .transpose()
        .map_err(SessionError::persistence)?;

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

/// Changeset for updating an agent session.
#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = agent_sessions)]
struct AgentSessionUpdate {
    pub end_sequence: Option<i64>,
    pub turn_ids: serde_json::Value,
    pub terminated_by_handoff: Option<uuid::Uuid>,
    pub context_snapshots: serde_json::Value,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub state: String,
}

/// Converts a domain `AgentSession` to update values.
fn session_to_update_values(session: &AgentSession) -> SessionResult<AgentSessionUpdate> {
    let turn_ids = serde_json::to_value(&session.turn_ids).map_err(SessionError::persistence)?;

    let context_snapshots =
        serde_json::to_value(&session.context_snapshots).map_err(SessionError::persistence)?;

    let end_sequence = session
        .end_sequence
        .map(|s| i64::try_from(s.value()))
        .transpose()
        .map_err(SessionError::persistence)?;

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
fn row_to_session(row: AgentSessionRow) -> SessionResult<AgentSession> {
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

/// Wrapper to convert session errors to repository result.
async fn run_blocking<F, T>(f: F) -> SessionResult<T>
where
    F: FnOnce() -> SessionResult<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(SessionError::persistence)?
}

/// Obtains a connection from the pool.
fn get_conn(
    pool: &PgPool,
) -> SessionResult<
    diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
> {
    pool.get().map_err(SessionError::persistence)
}
