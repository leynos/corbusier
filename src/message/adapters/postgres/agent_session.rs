//! `PostgreSQL` implementation of the `AgentSessionRepository` port using Diesel ORM.
//!
//! Provides production-grade persistence for agent sessions with JSONB storage
//! for turn IDs and context snapshots.
//!
//! Tenant isolation is enforced via `SET LOCAL app.tenant_id`, which sets a
//! `PostgreSQL` session variable scoped to the current transaction.  This
//! prepares the connection for Row-Level Security (RLS) policies once they
//! land in milestone 1.5.3.

use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde::Serialize;
use uuid::Uuid;

use super::blocking_helpers::{PgPool, get_conn_with, run_blocking_with};
use crate::context::RequestContext;
use crate::message::{
    adapters::models::{AgentSessionRow, NewAgentSession},
    adapters::schema::agent_sessions,
    domain::{
        AgentSession, AgentSessionId, AgentSessionState, ContextWindowSnapshot, ConversationId,
        HandoffId, SequenceNumber, TurnId,
    },
    ports::agent_session::{AgentSessionRepository, SessionError, SessionResult},
};

// ---------------------------------------------------------------------------
// Transaction helper
// ---------------------------------------------------------------------------

/// Adapter-local wrapper that satisfies Diesel's `From<diesel::result::Error>`
/// bound on [`PgConnection::transaction`] without leaking Diesel types into
/// the port layer.
enum TxError {
    Session(SessionError),
    Diesel(diesel::result::Error),
}

impl From<diesel::result::Error> for TxError {
    fn from(err: diesel::result::Error) -> Self {
        Self::Diesel(err)
    }
}

impl From<SessionError> for TxError {
    fn from(err: SessionError) -> Self {
        Self::Session(err)
    }
}

impl From<TxError> for SessionError {
    fn from(err: TxError) -> Self {
        match err {
            TxError::Session(e) => e,
            TxError::Diesel(e) => Self::persistence(e),
        }
    }
}

/// Runs `body` inside a transaction that first sets `app.tenant_id`.
fn with_tenant_tx<T, F>(conn: &mut PgConnection, tenant_id: Uuid, body: F) -> SessionResult<T>
where
    F: FnOnce(&mut PgConnection) -> SessionResult<T>,
{
    conn.transaction::<T, TxError, _>(|tx| {
        set_tenant_context(tx, tenant_id)?;
        body(tx).map_err(TxError::from)
    })
    .map_err(SessionError::from)
}

/// Sets the `PostgreSQL` session variable `app.tenant_id` for the current
/// transaction.
///
/// This prepares the connection for Row-Level Security (RLS) policies.
/// `SET LOCAL` scopes the variable to the enclosing transaction, so each
/// request gets an isolated tenant context.
///
/// # Security
///
/// UUID values are formatted using their canonical hyphenated representation
/// which contains only hexadecimal digits and hyphens, making SQL injection
/// impossible.
fn set_tenant_context(conn: &mut PgConnection, tenant_id: Uuid) -> Result<(), TxError> {
    diesel::sql_query(format!("SET LOCAL app.tenant_id = '{tenant_id}'")).execute(conn)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

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
    #[rustfmt::skip]
    pub const fn new(pool: PgPool) -> Self { Self { pool } }

    /// Executes a query inside a transaction with tenant context.
    async fn execute_query<F, T>(&self, tenant_id: Uuid, query_fn: F) -> SessionResult<T>
    where
        F: FnOnce(&mut PgConnection) -> SessionResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SessionError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id, query_fn)
            },
            SessionError::persistence,
        )
        .await
    }

    /// Execute a query that returns a single optional session.
    async fn find_one<F>(
        &self,
        tenant_id: Uuid,
        build_query: F,
    ) -> SessionResult<Option<AgentSession>>
    where
        F: FnOnce(agent_sessions::table) -> agent_sessions::BoxedQuery<'static, diesel::pg::Pg>
            + Send
            + 'static,
    {
        let sessions = self.find_many(tenant_id, build_query).await?;
        Ok(sessions.into_iter().next())
    }

    /// Execute a query that returns multiple sessions.
    async fn find_many<F>(
        &self,
        tenant_id: Uuid,
        build_query: F,
    ) -> SessionResult<Vec<AgentSession>>
    where
        F: FnOnce(agent_sessions::table) -> agent_sessions::BoxedQuery<'static, diesel::pg::Pg>
            + Send
            + 'static,
    {
        self.execute_query(tenant_id, move |conn| {
            let rows = build_query(agent_sessions::table)
                .select(AgentSessionRow::as_select())
                .load::<AgentSessionRow>(conn)
                .map_err(SessionError::persistence)?;

            rows.into_iter().map(row_to_session).collect()
        })
        .await
    }
}

#[async_trait]
impl AgentSessionRepository for PostgresAgentSessionRepository {
    async fn store(&self, ctx: &RequestContext, session: &AgentSession) -> SessionResult<()> {
        let pool = self.pool.clone();
        let tenant_id = ctx.tenant_id().into_inner();
        let new_session = session_to_new_row(session)?;
        let session_id = session.session_id;
        let conversation_id = session.conversation_id;
        let is_active = session.state == AgentSessionState::Active;

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SessionError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id, |tx| {
                    diesel::insert_into(agent_sessions::table)
                        .values(&new_session)
                        .execute(tx)
                        .map_err(|err| match err {
                            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                                SessionError::Duplicate(session_id)
                            }
                            _ => SessionError::persistence(err),
                        })?;

                    if is_active {
                        check_no_active_session(tx, conversation_id, Some(session_id))?;
                    }

                    Ok(())
                })
            },
            SessionError::persistence,
        )
        .await
    }

    async fn update(&self, ctx: &RequestContext, session: &AgentSession) -> SessionResult<()> {
        let pool = self.pool.clone();
        let tenant_id = ctx.tenant_id().into_inner();
        let session_id = session.session_id;
        let conversation_id = session.conversation_id;
        let is_active = session.state == AgentSessionState::Active;
        let updated = session_to_update_values(session)?;

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SessionError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id, |tx| {
                    let updated_rows = diesel::update(
                        agent_sessions::table
                            .filter(agent_sessions::id.eq(session_id.into_inner())),
                    )
                    .set(&updated)
                    .execute(tx)
                    .map_err(SessionError::persistence)?;

                    if updated_rows == 0 {
                        return Err(SessionError::NotFound(session_id));
                    }

                    if is_active {
                        check_no_active_session(tx, conversation_id, Some(session_id))?;
                    }

                    Ok(())
                })
            },
            SessionError::persistence,
        )
        .await
    }

    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        id: AgentSessionId,
    ) -> SessionResult<Option<AgentSession>> {
        let tenant_id = ctx.tenant_id().into_inner();
        let uuid = id.into_inner();

        self.find_one(tenant_id, move |table| {
            table.filter(agent_sessions::id.eq(uuid)).into_boxed()
        })
        .await
    }

    async fn find_active_for_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> SessionResult<Option<AgentSession>> {
        let tenant_id = ctx.tenant_id().into_inner();
        let uuid = conversation_id.into_inner();

        self.find_one(tenant_id, move |table| {
            table
                .filter(agent_sessions::conversation_id.eq(uuid))
                .filter(agent_sessions::state.eq(AgentSessionState::Active.as_str()))
                .into_boxed()
        })
        .await
    }

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> SessionResult<Vec<AgentSession>> {
        let tenant_id = ctx.tenant_id().into_inner();
        let uuid = conversation_id.into_inner();

        self.find_many(tenant_id, move |table| {
            table
                .filter(agent_sessions::conversation_id.eq(uuid))
                .order(agent_sessions::started_at.asc())
                .into_boxed()
        })
        .await
    }
}

/// Checks that no other active session exists for the given conversation.
///
/// When `exclude_id` is `Some`, the check ignores the session with that ID
/// (used during updates to allow re-saving the same active session).
fn check_no_active_session(
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

/// Converts a domain `AgentSession` to a `NewAgentSession` for insertion.
fn session_to_new_row(session: &AgentSession) -> SessionResult<NewAgentSession> {
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
