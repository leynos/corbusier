//! `PostgreSQL` implementation of the `ContextSnapshotPort` using Diesel ORM.
//!
//! Provides production-grade persistence for context window snapshots with JSONB
//! storage for message summaries and tool call references.

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
use diesel::prelude::*;

use super::blocking_helpers::{PgPool, get_conn_with, run_blocking_with};

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

    /// Generic helper to execute a database query with standard error handling.
    async fn execute_query<F, T>(&self, query_fn: F) -> SnapshotResult<T>
    where
        F: FnOnce(&mut PgConnection) -> SnapshotResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SnapshotError::persistence)?;
                query_fn(&mut conn)
            },
            SnapshotError::persistence,
        )
        .await
    }

    async fn find_one<F>(&self, build_query: F) -> SnapshotResult<Option<ContextWindowSnapshot>>
    where
        F: FnOnce(context_snapshots::table) -> context_snapshots::BoxedQuery<'static, Pg>
            + Send
            + 'static,
    {
        let snapshots = self.find_many(build_query).await?;
        Ok(snapshots.into_iter().next())
    }

    async fn find_many<F>(&self, build_query: F) -> SnapshotResult<Vec<ContextWindowSnapshot>>
    where
        F: FnOnce(context_snapshots::table) -> context_snapshots::BoxedQuery<'static, Pg>
            + Send
            + 'static,
    {
        self.execute_query(move |conn| {
            let rows = build_query(context_snapshots::table)
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
    async fn store_snapshot(&self, snapshot: &ContextWindowSnapshot) -> SnapshotResult<()> {
        let pool = self.pool.clone();
        let new_snapshot = snapshot_to_new_row(snapshot)?;
        let snapshot_id = snapshot.snapshot_id;

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SnapshotError::persistence)?;

                let inserted = diesel::insert_into(context_snapshots::table)
                    .values(&new_snapshot)
                    .on_conflict(context_snapshots::id)
                    .do_nothing()
                    .execute(&mut conn)
                    .map_err(SnapshotError::persistence)?;

                if inserted == 0 {
                    return Err(SnapshotError::Duplicate(snapshot_id));
                }

                Ok(())
            },
            SnapshotError::persistence,
        )
        .await
    }

    async fn find_by_id(
        &self,
        snapshot_id: uuid::Uuid,
    ) -> SnapshotResult<Option<ContextWindowSnapshot>> {
        self.find_one(move |table| {
            table
                .filter(context_snapshots::id.eq(snapshot_id))
                .into_boxed()
        })
        .await
    }

    async fn find_snapshots_for_session(
        &self,
        session_id: AgentSessionId,
    ) -> SnapshotResult<Vec<ContextWindowSnapshot>> {
        let uuid = session_id.into_inner();

        self.find_many(move |table| {
            table
                .filter(context_snapshots::session_id.eq(uuid))
                .order(context_snapshots::captured_at.asc())
                .into_boxed()
        })
        .await
    }

    async fn find_latest_snapshot(
        &self,
        conversation_id: ConversationId,
    ) -> SnapshotResult<Option<ContextWindowSnapshot>> {
        let uuid = conversation_id.into_inner();

        self.find_one(move |table| {
            table
                .filter(context_snapshots::conversation_id.eq(uuid))
                .order(context_snapshots::captured_at.desc())
                .into_boxed()
        })
        .await
    }
}

/// Converts a domain `ContextWindowSnapshot` to a `NewContextSnapshot` for insertion.
fn snapshot_to_new_row(snapshot: &ContextWindowSnapshot) -> SnapshotResult<NewContextSnapshot> {
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
