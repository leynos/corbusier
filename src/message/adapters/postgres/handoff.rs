//! `PostgreSQL` implementation of the `AgentHandoffPort` using Diesel ORM.
//!
//! Provides production-grade persistence for handoff metadata with JSONB storage
//! for tool call references.
//!
//! Tenant context is propagated via `SET LOCAL app.tenant_id`, which sets a
//! `PostgreSQL` session variable scoped to the current transaction.  This
//! prepares the connection for Row-Level Security (RLS) policies but does
//! not enforce row isolation by itself; actual enforcement requires RLS
//! policies on the `handoffs` table, which land in milestone 1.5.3.

use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use mockable::DefaultClock;

use crate::context::{RequestContext, TenantId};
use crate::message::{
    adapters::models::{HandoffRow, NewHandoff},
    adapters::schema::handoffs,
    domain::{
        AgentSessionId, ConversationId, HandoffId, HandoffMetadata, HandoffParams, HandoffStatus,
        ToolCallReference, TurnId,
    },
    ports::handoff::{AgentHandoffPort, HandoffError, HandoffResult, InitiateHandoffParams},
};

use super::blocking_helpers::{PgPool, get_conn_with, run_blocking_with};
use super::tenant_tx::{
    FromTxError, TxError, ensure_tenant_exists, with_tenant_read_tx, with_tenant_tx,
};

// ---------------------------------------------------------------------------
// Error bridging for the shared transaction helper
// ---------------------------------------------------------------------------

impl FromTxError<Self> for HandoffError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(e) => e,
            TxError::Diesel(e) => Self::persistence(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

/// `PostgreSQL` implementation of [`AgentHandoffPort`].
///
/// Uses Diesel ORM with connection pooling via r2d2. Thread-safe for
/// concurrent access.
#[derive(Debug, Clone)]
pub struct PostgresHandoffAdapter {
    pool: PgPool,
}

impl PostgresHandoffAdapter {
    /// Creates a new adapter with the given connection pool.
    #[must_use]
    #[rustfmt::skip]
    pub const fn new(pool: PgPool) -> Self { Self { pool } }

    /// Executes a write query that may create the tenant row before use.
    async fn execute_query_with_bootstrap<F, T>(
        &self,
        tenant_id: TenantId,
        query_fn: F,
    ) -> HandoffResult<T>
    where
        F: FnOnce(&mut PgConnection) -> HandoffResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, HandoffError::persistence)?;
                ensure_tenant_exists(&mut conn, tenant_id.into_inner())
                    .map_err(HandoffError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), query_fn)
            },
            HandoffError::persistence,
        )
        .await
    }

    /// Executes a write query inside a transaction with tenant context.
    async fn execute_query<F, T>(&self, tenant_id: TenantId, query_fn: F) -> HandoffResult<T>
    where
        F: FnOnce(&mut PgConnection) -> HandoffResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, HandoffError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), query_fn)
            },
            HandoffError::persistence,
        )
        .await
    }

    /// Executes a read-only query inside a transaction with tenant context.
    async fn execute_read_query<F, T>(&self, tenant_id: TenantId, query_fn: F) -> HandoffResult<T>
    where
        F: FnOnce(&mut PgConnection) -> HandoffResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, HandoffError::persistence)?;
                with_tenant_read_tx(&mut conn, tenant_id.into_inner(), query_fn)
            },
            HandoffError::persistence,
        )
        .await
    }
}

#[async_trait]
impl AgentHandoffPort for PostgresHandoffAdapter {
    async fn initiate_handoff(
        &self,
        ctx: &RequestContext,
        params: InitiateHandoffParams<'_>,
    ) -> HandoffResult<HandoffMetadata> {
        let tenant_id = ctx.tenant_id();
        let source_session_id = params.source_session.session_id;
        let source_agent = params.source_session.agent_backend.clone();
        let owned_target_agent = params.target_agent.to_owned();
        let owned_reason = params.reason.map(String::from);
        let clock = DefaultClock;

        let handoff_params = HandoffParams::new(
            source_session_id,
            params.prior_turn_id,
            &source_agent,
            &owned_target_agent,
        );
        let mut handoff = HandoffMetadata::new(handoff_params, &clock);

        if let Some(r) = owned_reason {
            handoff = handoff.with_reason(r);
        }

        let new_handoff = handoff_to_new_row(&handoff, params.conversation_id, tenant_id)?;

        self.execute_query_with_bootstrap(tenant_id, move |conn| {
            diesel::insert_into(handoffs::table)
                .values(&new_handoff)
                .execute(conn)
                .map_err(HandoffError::persistence)?;

            Ok(handoff)
        })
        .await
    }

    async fn complete_handoff(
        &self,
        ctx: &RequestContext,
        handoff_id: HandoffId,
        target_session_id: AgentSessionId,
    ) -> HandoffResult<HandoffMetadata> {
        let tenant_id = ctx.tenant_id();
        let clock = DefaultClock;

        self.execute_query(tenant_id, move |conn| {
            // Lock the row for the duration of the transaction to
            // prevent concurrent state transitions from interleaving.
            let row = handoffs::table
                .filter(handoffs::id.eq(handoff_id.into_inner()))
                .filter(handoffs::tenant_id.eq(tenant_id.into_inner()))
                .select(HandoffRow::as_select())
                .for_update()
                .first::<HandoffRow>(conn)
                .optional()
                .map_err(HandoffError::persistence)?
                .ok_or(HandoffError::NotFound(handoff_id))?;

            let mut handoff = row_to_handoff(row)?;

            if handoff.is_terminal() {
                return Err(HandoffError::invalid_transition(
                    handoff.status,
                    HandoffStatus::Completed,
                ));
            }

            handoff = handoff.complete(target_session_id, &clock);
            let completed_at = handoff.completed_at;

            diesel::update(
                handoffs::table
                    .filter(handoffs::id.eq(handoff_id.into_inner()))
                    .filter(handoffs::tenant_id.eq(tenant_id.into_inner())),
            )
            .set((
                handoffs::target_session_id.eq(target_session_id.into_inner()),
                handoffs::completed_at.eq(completed_at),
                handoffs::status.eq(HandoffStatus::Completed.as_str()),
            ))
            .execute(conn)
            .map_err(HandoffError::persistence)?;

            Ok(handoff)
        })
        .await
    }

    async fn cancel_handoff(
        &self,
        ctx: &RequestContext,
        handoff_id: HandoffId,
        reason: Option<&str>,
    ) -> HandoffResult<()> {
        let tenant_id = ctx.tenant_id();
        let owned_reason = reason.map(str::to_owned);

        self.execute_query(tenant_id, move |conn| {
            // Lock the row for the duration of the transaction to
            // prevent concurrent state transitions from interleaving.
            let row = handoffs::table
                .filter(handoffs::id.eq(handoff_id.into_inner()))
                .filter(handoffs::tenant_id.eq(tenant_id.into_inner()))
                .select(HandoffRow::as_select())
                .for_update()
                .first::<HandoffRow>(conn)
                .optional()
                .map_err(HandoffError::persistence)?
                .ok_or(HandoffError::NotFound(handoff_id))?;

            let status =
                HandoffStatus::try_from(row.status.as_str()).map_err(HandoffError::persistence)?;

            if status.is_terminal() {
                return Err(HandoffError::invalid_transition(
                    status,
                    HandoffStatus::Cancelled,
                ));
            }

            diesel::update(
                handoffs::table
                    .filter(handoffs::id.eq(handoff_id.into_inner()))
                    .filter(handoffs::tenant_id.eq(tenant_id.into_inner())),
            )
            .set((
                handoffs::status.eq(HandoffStatus::Cancelled.as_str()),
                handoffs::reason.eq(owned_reason.or(row.reason)),
            ))
            .execute(conn)
            .map_err(HandoffError::persistence)?;

            Ok(())
        })
        .await
    }

    async fn find_handoff(
        &self,
        ctx: &RequestContext,
        handoff_id: HandoffId,
    ) -> HandoffResult<Option<HandoffMetadata>> {
        let tenant_id = ctx.tenant_id();
        let uuid = handoff_id.into_inner();

        self.execute_read_query(tenant_id, move |conn| {
            handoffs::table
                .filter(handoffs::id.eq(uuid))
                .filter(handoffs::tenant_id.eq(tenant_id.into_inner()))
                .select(HandoffRow::as_select())
                .first::<HandoffRow>(conn)
                .optional()
                .map_err(HandoffError::persistence)?
                .map(row_to_handoff)
                .transpose()
        })
        .await
    }

    async fn list_handoffs_for_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HandoffResult<Vec<HandoffMetadata>> {
        let tenant_id = ctx.tenant_id();
        let uuid = conversation_id.into_inner();

        self.execute_read_query(tenant_id, move |conn| {
            let rows = handoffs::table
                .filter(handoffs::tenant_id.eq(tenant_id.into_inner()))
                .filter(handoffs::conversation_id.eq(uuid))
                .select(HandoffRow::as_select())
                .order(handoffs::initiated_at.asc())
                .load::<HandoffRow>(conn)
                .map_err(HandoffError::persistence)?;

            rows.into_iter().map(row_to_handoff).collect()
        })
        .await
    }
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

/// Converts a domain `HandoffMetadata` to a `NewHandoff` for insertion.
fn handoff_to_new_row(
    handoff: &HandoffMetadata,
    conversation_id: ConversationId,
    tenant_id: TenantId,
) -> HandoffResult<NewHandoff> {
    let triggering_tool_calls =
        serde_json::to_value(&handoff.triggering_tool_calls).map_err(HandoffError::persistence)?;

    Ok(NewHandoff {
        id: handoff.handoff_id.into_inner(),
        tenant_id: tenant_id.into_inner(),
        source_session_id: handoff.source_session_id.into_inner(),
        conversation_id: conversation_id.into_inner(),
        target_session_id: handoff.target_session_id.map(AgentSessionId::into_inner),
        prior_turn_id: handoff.prior_turn_id.into_inner(),
        triggering_tool_calls,
        source_agent: handoff.source_agent.clone(),
        target_agent: handoff.target_agent.clone(),
        reason: handoff.reason.clone(),
        initiated_at: handoff.initiated_at,
        completed_at: handoff.completed_at,
        status: handoff.status.as_str().to_owned(),
    })
}

/// Converts a database row to a domain `HandoffMetadata`.
fn row_to_handoff(row: HandoffRow) -> HandoffResult<HandoffMetadata> {
    let triggering_tool_calls: Vec<ToolCallReference> =
        serde_json::from_value(row.triggering_tool_calls).map_err(HandoffError::persistence)?;

    let status = HandoffStatus::try_from(row.status.as_str()).map_err(HandoffError::persistence)?;

    Ok(HandoffMetadata {
        handoff_id: HandoffId::from_uuid(row.id),
        source_session_id: AgentSessionId::from_uuid(row.source_session_id),
        target_session_id: row.target_session_id.map(AgentSessionId::from_uuid),
        prior_turn_id: TurnId::from_uuid(row.prior_turn_id),
        triggering_tool_calls,
        source_agent: row.source_agent,
        target_agent: row.target_agent,
        reason: row.reason,
        initiated_at: row.initiated_at,
        completed_at: row.completed_at,
        status,
    })
}
