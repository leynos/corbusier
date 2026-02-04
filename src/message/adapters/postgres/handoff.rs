//! `PostgreSQL` implementation of the `AgentHandoffPort` using Diesel ORM.
//!
//! Provides production-grade persistence for handoff metadata with JSONB storage
//! for tool call references.

use async_trait::async_trait;
use diesel::prelude::*;
use mockable::DefaultClock;

use crate::message::{
    adapters::models::{HandoffRow, NewHandoff},
    adapters::schema::{agent_sessions, handoffs},
    domain::{
        AgentSessionId, ConversationId, HandoffId, HandoffMetadata, HandoffParams, HandoffStatus,
        ToolCallReference, TurnId,
    },
    ports::handoff::{AgentHandoffPort, HandoffError, HandoffResult, InitiateHandoffParams},
};

use super::blocking_helpers::{PgPool, get_conn_with, run_blocking_with};

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
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AgentHandoffPort for PostgresHandoffAdapter {
    async fn initiate_handoff(
        &self,
        params: InitiateHandoffParams<'_>,
    ) -> HandoffResult<HandoffMetadata> {
        let pool = self.pool.clone();
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

        let new_handoff = handoff_to_new_row(&handoff)?;

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, HandoffError::persistence)?;

                diesel::insert_into(handoffs::table)
                    .values(&new_handoff)
                    .execute(&mut conn)
                    .map_err(HandoffError::persistence)?;

                Ok(handoff)
            },
            HandoffError::persistence,
        )
        .await
    }

    async fn complete_handoff(
        &self,
        handoff_id: HandoffId,
        target_session_id: AgentSessionId,
    ) -> HandoffResult<HandoffMetadata> {
        let pool = self.pool.clone();
        let clock = DefaultClock;

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, HandoffError::persistence)?;

                // Find the handoff
                let row = handoffs::table
                    .filter(handoffs::id.eq(handoff_id.into_inner()))
                    .select(HandoffRow::as_select())
                    .first::<HandoffRow>(&mut conn)
                    .optional()
                    .map_err(HandoffError::persistence)?
                    .ok_or(HandoffError::NotFound(handoff_id))?;

                let mut handoff = row_to_handoff(row)?;

                // Check if already completed
                if handoff.is_terminal() {
                    return Err(HandoffError::invalid_transition(
                        handoff.status,
                        HandoffStatus::Completed,
                    ));
                }

                // Complete the handoff
                handoff = handoff.complete(target_session_id, &clock);
                let completed_at = handoff.completed_at;

                // Update in database
                diesel::update(handoffs::table.filter(handoffs::id.eq(handoff_id.into_inner())))
                    .set((
                        handoffs::target_session_id.eq(target_session_id.into_inner()),
                        handoffs::completed_at.eq(completed_at),
                        handoffs::status.eq(HandoffStatus::Completed.as_str()),
                    ))
                    .execute(&mut conn)
                    .map_err(HandoffError::persistence)?;

                Ok(handoff)
            },
            HandoffError::persistence,
        )
        .await
    }

    async fn cancel_handoff(
        &self,
        handoff_id: HandoffId,
        reason: Option<&str>,
    ) -> HandoffResult<()> {
        let pool = self.pool.clone();
        let owned_reason = reason.map(str::to_owned);

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, HandoffError::persistence)?;

                // Check if handoff exists and is not terminal
                let row = handoffs::table
                    .filter(handoffs::id.eq(handoff_id.into_inner()))
                    .select(HandoffRow::as_select())
                    .first::<HandoffRow>(&mut conn)
                    .optional()
                    .map_err(HandoffError::persistence)?
                    .ok_or(HandoffError::NotFound(handoff_id))?;

                let status = HandoffStatus::try_from(row.status.as_str())
                    .map_err(HandoffError::persistence)?;

                if status.is_terminal() {
                    return Err(HandoffError::invalid_transition(
                        status,
                        HandoffStatus::Cancelled,
                    ));
                }

                // Cancel the handoff
                diesel::update(handoffs::table.filter(handoffs::id.eq(handoff_id.into_inner())))
                    .set((
                        handoffs::status.eq(HandoffStatus::Cancelled.as_str()),
                        handoffs::reason.eq(owned_reason),
                    ))
                    .execute(&mut conn)
                    .map_err(HandoffError::persistence)?;

                Ok(())
            },
            HandoffError::persistence,
        )
        .await
    }

    async fn find_handoff(&self, handoff_id: HandoffId) -> HandoffResult<Option<HandoffMetadata>> {
        let pool = self.pool.clone();
        let uuid = handoff_id.into_inner();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, HandoffError::persistence)?;

                handoffs::table
                    .filter(handoffs::id.eq(uuid))
                    .select(HandoffRow::as_select())
                    .first::<HandoffRow>(&mut conn)
                    .optional()
                    .map_err(HandoffError::persistence)?
                    .map(row_to_handoff)
                    .transpose()
            },
            HandoffError::persistence,
        )
        .await
    }

    async fn list_handoffs_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> HandoffResult<Vec<HandoffMetadata>> {
        let pool = self.pool.clone();
        let uuid = conversation_id.into_inner();

        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, HandoffError::persistence)?;

                // Join with agent_sessions to find handoffs for this conversation
                let rows = handoffs::table
                    .inner_join(
                        agent_sessions::table
                            .on(handoffs::source_session_id.eq(agent_sessions::id)),
                    )
                    .filter(agent_sessions::conversation_id.eq(uuid))
                    .select(HandoffRow::as_select())
                    .order(handoffs::initiated_at.asc())
                    .load::<HandoffRow>(&mut conn)
                    .map_err(HandoffError::persistence)?;

                rows.into_iter().map(row_to_handoff).collect()
            },
            HandoffError::persistence,
        )
        .await
    }
}

/// Converts a domain `HandoffMetadata` to a `NewHandoff` for insertion.
fn handoff_to_new_row(handoff: &HandoffMetadata) -> HandoffResult<NewHandoff> {
    let triggering_tool_calls =
        serde_json::to_value(&handoff.triggering_tool_calls).map_err(HandoffError::persistence)?;

    Ok(NewHandoff {
        id: handoff.handoff_id.into_inner(),
        source_session_id: handoff.source_session_id.into_inner(),
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
