//! `PostgreSQL` implementation of the `ContextSnapshotPort` using Diesel ORM.
//!
//! Provides production-grade persistence for context window snapshots with JSONB
//! storage for message summaries and tool call references.

use crate::context::{RequestContext, TenantId};
use crate::message::{
    adapters::models::{ContextSnapshotRow, NewContextSnapshot},
    adapters::schema::context_snapshots,
    domain::{
        AgentSessionId, ContextWindowSnapshot, ConversationId, MessageSummary, SequenceNumber,
        SequenceRange, SnapshotType, ToolCallReference,
    },
    ports::context_snapshot::{ContextSnapshotPort, SnapshotError, SnapshotResult},
};
use async_trait::async_trait;
use diesel::pg::Pg;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use super::blocking_helpers::{PgPool, get_conn_with, run_blocking_with};
use super::tenant_tx::{FromTxError, TxError, with_tenant_read_tx, with_tenant_tx};

impl FromTxError<Self> for SnapshotError {
    fn from_tx_error(tx_err: TxError<Self>) -> Self {
        match tx_err {
            TxError::Domain(domain_err) => domain_err,
            TxError::Diesel(diesel_err) => Self::persistence(diesel_err),
        }
    }
}

/// `PostgreSQL` implementation of [`ContextSnapshotPort`].
///
/// Uses Diesel ORM with connection pooling via r2d2. Thread-safe for
/// concurrent access.
#[derive(Debug, Clone)]
pub struct PostgresContextSnapshotAdapter {
    pool: PgPool,
}

impl PostgresContextSnapshotAdapter {
    /// Creates a new adapter with the given connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Generic helper to execute a read-only query with standard error handling.
    async fn execute_read_query<F, T>(&self, tenant_id: TenantId, query_fn: F) -> SnapshotResult<T>
    where
        F: FnOnce(&mut PgConnection) -> SnapshotResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SnapshotError::persistence)?;
                with_tenant_read_tx(&mut conn, tenant_id.into_inner(), query_fn)
            },
            SnapshotError::persistence,
        )
        .await
    }

    async fn find_one<F>(
        &self,
        tenant_id: TenantId,
        build_query: F,
    ) -> SnapshotResult<Option<ContextWindowSnapshot>>
    where
        F: FnOnce(
                context_snapshots::BoxedQuery<'static, Pg>,
            ) -> context_snapshots::BoxedQuery<'static, Pg>
            + Send
            + 'static,
    {
        let tenant_uuid = tenant_id.into_inner();
        self.execute_read_query(tenant_id, move |conn| {
            let base = context_snapshots::table
                .filter(context_snapshots::tenant_id.eq(tenant_uuid))
                .into_boxed();
            let row_opt = build_query(base)
                .select(ContextSnapshotRow::as_select())
                .limit(1)
                .first::<ContextSnapshotRow>(conn)
                .optional()
                .map_err(SnapshotError::persistence)?;

            row_opt.map(row_to_snapshot).transpose()
        })
        .await
    }

    async fn find_many<F>(
        &self,
        tenant_id: TenantId,
        build_query: F,
    ) -> SnapshotResult<Vec<ContextWindowSnapshot>>
    where
        F: FnOnce(
                context_snapshots::BoxedQuery<'static, Pg>,
            ) -> context_snapshots::BoxedQuery<'static, Pg>
            + Send
            + 'static,
    {
        let tenant_uuid = tenant_id.into_inner();
        self.execute_read_query(tenant_id, move |conn| {
            let base = context_snapshots::table
                .filter(context_snapshots::tenant_id.eq(tenant_uuid))
                .into_boxed();
            let rows = build_query(base)
                .select(ContextSnapshotRow::as_select())
                .load::<ContextSnapshotRow>(conn)
                .map_err(SnapshotError::persistence)?;

            rows.into_iter().map(row_to_snapshot).collect()
        })
        .await
    }
}

#[async_trait]
impl ContextSnapshotPort for PostgresContextSnapshotAdapter {
    async fn store_snapshot(
        &self,
        ctx: &RequestContext,
        snapshot: &ContextWindowSnapshot,
    ) -> SnapshotResult<()> {
        let tenant_id = ctx.tenant_id();
        let pool = self.pool.clone();
        let new_snapshot = snapshot_to_new_row(snapshot, tenant_id.into_inner())?;
        let snapshot_id = snapshot.snapshot_id;

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SnapshotError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), |tx| {
                    let inserted = diesel::insert_into(context_snapshots::table)
                        .values(&new_snapshot)
                        .on_conflict(context_snapshots::id)
                        .do_nothing()
                        .execute(tx)
                        .map_err(SnapshotError::persistence)?;

                    if inserted == 0 {
                        return Err(SnapshotError::Duplicate(snapshot_id));
                    }

                    Ok(())
                })
            },
            SnapshotError::persistence,
        )
        .await
    }

    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        snapshot_id: uuid::Uuid,
    ) -> SnapshotResult<Option<ContextWindowSnapshot>> {
        let tenant_id = ctx.tenant_id();
        self.find_one(tenant_id, move |q| {
            q.filter(context_snapshots::id.eq(snapshot_id))
        })
        .await
    }

    async fn find_snapshots_for_session(
        &self,
        ctx: &RequestContext,
        session_id: AgentSessionId,
    ) -> SnapshotResult<Vec<ContextWindowSnapshot>> {
        let tenant_id = ctx.tenant_id();
        let uuid = session_id.into_inner();

        self.find_many(tenant_id, move |q| {
            q.filter(context_snapshots::session_id.eq(uuid))
                .order(context_snapshots::captured_at.asc())
        })
        .await
    }

    async fn find_latest_snapshot(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> SnapshotResult<Option<ContextWindowSnapshot>> {
        let tenant_id = ctx.tenant_id();
        let uuid = conversation_id.into_inner();

        self.find_one(tenant_id, move |q| {
            q.filter(context_snapshots::conversation_id.eq(uuid))
                .order(context_snapshots::captured_at.desc())
        })
        .await
    }
}

/// Converts a domain `ContextWindowSnapshot` to a `NewContextSnapshot` for insertion.
fn snapshot_to_new_row(
    snapshot: &ContextWindowSnapshot,
    tenant_id: uuid::Uuid,
) -> SnapshotResult<NewContextSnapshot> {
    let message_summary =
        serde_json::to_value(snapshot.message_summary).map_err(SnapshotError::persistence)?;

    let visible_tool_calls =
        serde_json::to_value(&snapshot.visible_tool_calls).map_err(SnapshotError::persistence)?;

    let sequence_start =
        i64::try_from(snapshot.sequence_range.start.value()).map_err(SnapshotError::persistence)?;

    let sequence_end =
        i64::try_from(snapshot.sequence_range.end.value()).map_err(SnapshotError::persistence)?;

    let token_estimate = snapshot
        .token_estimate
        .map(i64::try_from)
        .transpose()
        .map_err(SnapshotError::persistence)?;

    Ok(NewContextSnapshot {
        id: snapshot.snapshot_id,
        tenant_id,
        conversation_id: snapshot.conversation_id.into_inner(),
        session_id: snapshot.session_id.into_inner(),
        sequence_start,
        sequence_end,
        message_summary,
        visible_tool_calls,
        token_estimate,
        captured_at: snapshot.captured_at,
        snapshot_type: snapshot.snapshot_type.as_str().to_owned(),
    })
}

/// Converts a database row to a domain `ContextWindowSnapshot`.
fn row_to_snapshot(row: ContextSnapshotRow) -> SnapshotResult<ContextWindowSnapshot> {
    let message_summary: MessageSummary =
        serde_json::from_value(row.message_summary).map_err(SnapshotError::persistence)?;

    let visible_tool_calls: Vec<ToolCallReference> =
        serde_json::from_value(row.visible_tool_calls).map_err(SnapshotError::persistence)?;

    let start = u64::try_from(row.sequence_start).map_err(SnapshotError::persistence)?;

    let end = u64::try_from(row.sequence_end).map_err(SnapshotError::persistence)?;

    let token_estimate = row
        .token_estimate
        .map(u64::try_from)
        .transpose()
        .map_err(SnapshotError::persistence)?;

    let snapshot_type =
        SnapshotType::try_from(row.snapshot_type.as_str()).map_err(SnapshotError::persistence)?;

    Ok(ContextWindowSnapshot {
        snapshot_id: row.id,
        conversation_id: ConversationId::from_uuid(row.conversation_id),
        session_id: AgentSessionId::from_uuid(row.session_id),
        sequence_range: SequenceRange::new(SequenceNumber::new(start), SequenceNumber::new(end)),
        message_summary,
        visible_tool_calls,
        token_estimate,
        captured_at: row.captured_at,
        snapshot_type,
    })
}
