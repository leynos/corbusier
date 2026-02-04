//! `PostgreSQL` implementation of the `ContextSnapshotPort` using Diesel ORM.
//!
//! Provides production-grade persistence for context window snapshots with JSONB
//! storage for message summaries and tool call references.

use async_trait::async_trait;
use diesel::prelude::*;
use mockable::DefaultClock;

use crate::message::{
    adapters::models::{ContextSnapshotRow, NewContextSnapshot},
    adapters::schema::context_snapshots,
    domain::{
        AgentSessionId, ContextWindowSnapshot, ConversationId, MessageSummary, SequenceNumber,
        SequenceRange, SnapshotType, ToolCallReference,
    },
    ports::context_snapshot::{
        CaptureSnapshotParams, ContextSnapshotPort, SnapshotError, SnapshotResult,
        build_default_snapshot,
    },
};

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
}

#[async_trait]
impl ContextSnapshotPort for PostgresContextSnapshotAdapter {
    async fn capture_snapshot(
        &self,
        params: CaptureSnapshotParams,
    ) -> SnapshotResult<ContextWindowSnapshot> {
        // This is a simplified implementation. A full implementation would
        // query messages to calculate actual message summaries.
        let clock = DefaultClock;

        let snapshot = build_default_snapshot(&params, &clock);

        // Store the snapshot
        self.store_snapshot(&snapshot).await?;

        Ok(snapshot)
    }

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
        let pool = self.pool.clone();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SnapshotError::persistence)?;

                context_snapshots::table
                    .filter(context_snapshots::id.eq(snapshot_id))
                    .select(ContextSnapshotRow::as_select())
                    .first::<ContextSnapshotRow>(&mut conn)
                    .optional()
                    .map_err(SnapshotError::persistence)?
                    .map(row_to_snapshot)
                    .transpose()
            },
            SnapshotError::persistence,
        )
        .await
    }

    async fn find_snapshots_for_session(
        &self,
        session_id: AgentSessionId,
    ) -> SnapshotResult<Vec<ContextWindowSnapshot>> {
        let pool = self.pool.clone();
        let uuid = session_id.into_inner();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SnapshotError::persistence)?;

                let rows = context_snapshots::table
                    .filter(context_snapshots::session_id.eq(uuid))
                    .order(context_snapshots::captured_at.asc())
                    .select(ContextSnapshotRow::as_select())
                    .load::<ContextSnapshotRow>(&mut conn)
                    .map_err(SnapshotError::persistence)?;

                rows.into_iter().map(row_to_snapshot).collect()
            },
            SnapshotError::persistence,
        )
        .await
    }

    async fn find_latest_snapshot(
        &self,
        conversation_id: ConversationId,
    ) -> SnapshotResult<Option<ContextWindowSnapshot>> {
        let pool = self.pool.clone();
        let uuid = conversation_id.into_inner();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, SnapshotError::persistence)?;

                context_snapshots::table
                    .filter(context_snapshots::conversation_id.eq(uuid))
                    .order(context_snapshots::captured_at.desc())
                    .select(ContextSnapshotRow::as_select())
                    .first::<ContextSnapshotRow>(&mut conn)
                    .optional()
                    .map_err(SnapshotError::persistence)?
                    .map(row_to_snapshot)
                    .transpose()
            },
            SnapshotError::persistence,
        )
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
