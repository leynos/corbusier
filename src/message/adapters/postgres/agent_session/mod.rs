//! `PostgreSQL` implementation of the `AgentSessionRepository` port using Diesel ORM.
//!
//! Provides production-grade persistence for agent sessions with JSONB storage
//! for turn IDs and context snapshots.
//!
//! Tenant context is propagated via `SET LOCAL app.tenant_id`, which sets a
//! `PostgreSQL` session variable scoped to the current transaction.  This
//! prepares the connection for Row-Level Security (RLS) policies but does
//! not enforce row isolation by itself; actual enforcement requires RLS
//! policies on the `agent_sessions` table, which land in milestone 1.5.3.

mod constraint_helpers;
mod row_mapping;

use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use super::blocking_helpers::{PgPool, get_conn_with, run_blocking_with};
use super::tenant_tx::with_tenant_tx;
use crate::context::{RequestContext, TenantId};
use crate::message::{
    adapters::models::AgentSessionRow,
    adapters::schema::agent_sessions,
    domain::{AgentSession, AgentSessionId, AgentSessionState, ConversationId},
    ports::agent_session::{AgentSessionRepository, SessionError, SessionResult},
};

use constraint_helpers::{check_no_active_session, map_insert_error, map_update_error};
use row_mapping::{row_to_session, session_to_new_row, session_to_update_values};

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
    async fn execute_query<F, T>(&self, tenant_id: TenantId, query_fn: F) -> SessionResult<T>
    where
        F: FnOnce(&mut PgConnection) -> SessionResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SessionError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), query_fn)
            },
            SessionError::persistence,
        )
        .await
    }

    /// Execute a query that returns a single optional session.
    async fn find_one<F>(
        &self,
        tenant_id: TenantId,
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
        tenant_id: TenantId,
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
        let tenant_id = ctx.tenant_id();
        let new_session = session_to_new_row(session, tenant_id.into_inner())?;
        let session_id = session.session_id;
        let conversation_id = session.conversation_id;
        let is_active = session.state == AgentSessionState::Active;

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SessionError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), |tx| {
                    diesel::insert_into(agent_sessions::table)
                        .values(&new_session)
                        .execute(tx)
                        .map_err(|err| map_insert_error(err, session_id, conversation_id))?;

                    if is_active {
                        check_no_active_session(
                            tx,
                            tenant_id.into_inner(),
                            conversation_id,
                            Some(session_id),
                        )?;
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
        let tenant_id = ctx.tenant_id();
        let session_id = session.session_id;
        let conversation_id = session.conversation_id;
        let is_active = session.state == AgentSessionState::Active;
        let updated = session_to_update_values(session)?;

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SessionError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), |tx| {
                    let updated_rows = diesel::update(
                        agent_sessions::table
                            .filter(agent_sessions::id.eq(session_id.into_inner()))
                            .filter(agent_sessions::tenant_id.eq(tenant_id.into_inner())),
                    )
                    .set(&updated)
                    .execute(tx)
                    .map_err(|err| map_update_error(err, session_id, conversation_id))?;

                    if updated_rows == 0 {
                        return Err(SessionError::NotFound(session_id));
                    }

                    if is_active {
                        check_no_active_session(
                            tx,
                            tenant_id.into_inner(),
                            conversation_id,
                            Some(session_id),
                        )?;
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
        let tenant_id = ctx.tenant_id();
        let tenant_uuid = tenant_id.into_inner();
        let uuid = id.into_inner();

        self.find_one(tenant_id, move |table| {
            table
                .filter(agent_sessions::tenant_id.eq(tenant_uuid))
                .filter(agent_sessions::id.eq(uuid))
                .into_boxed()
        })
        .await
    }

    async fn find_active_for_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> SessionResult<Option<AgentSession>> {
        let sessions = self.find_by_conversation(ctx, conversation_id).await?;
        Ok(sessions
            .into_iter()
            .find(|s| s.state == AgentSessionState::Active))
    }

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> SessionResult<Vec<AgentSession>> {
        let tenant_id = ctx.tenant_id();
        let tenant_uuid = tenant_id.into_inner();
        let uuid = conversation_id.into_inner();

        self.find_many(tenant_id, move |table| {
            table
                .filter(agent_sessions::tenant_id.eq(tenant_uuid))
                .filter(agent_sessions::conversation_id.eq(uuid))
                .order(agent_sessions::started_at.asc())
                .into_boxed()
        })
        .await
    }
}
