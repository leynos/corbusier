//! SQL helper functions for tenant schema constraint tests.
//!
//! Provides raw SQL insert functions that bypass ORM validation to test
//! database-level foreign key constraints.

use corbusier::context::TenantId;
use corbusier::message::domain::{AgentSessionId, ConversationId, HandoffId, MessageId};
use corbusier::task::domain::TaskId;
use corbusier::test_support::bootstrap_tenant_row as ensure_tenant_exists;
use diesel::RunQueryDsl;
use diesel::pg::PgConnection;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(crate) struct HandoffInsert {
    pub handoff: HandoffId,
    pub tenant: TenantId,
    pub source_session: AgentSessionId,
    pub conversation: ConversationId,
    pub target_session: Option<AgentSessionId>,
}

#[derive(Clone, Copy)]
pub(crate) struct ContextSnapshotInsert {
    pub snapshot: Uuid,
    pub tenant: TenantId,
    pub conversation: ConversationId,
    pub session: AgentSessionId,
}

#[derive(Clone, Copy)]
pub(crate) struct AgentSessionInsert {
    pub session: AgentSessionId,
    pub tenant: TenantId,
    pub conversation: ConversationId,
    pub is_active: bool,
}

pub(crate) fn insert_task(
    conn: &mut PgConnection,
    task_id: TaskId,
    tenant_id: TenantId,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, tenant_id)?;
    let task_uuid = task_id.into_inner();
    let upper_bits = task_uuid.as_u128() >> 64;
    let issue_number = u64::try_from(upper_bits).unwrap_or(1).max(1);
    diesel::sql_query(concat!(
        "INSERT INTO tasks (id, tenant_id, origin, state, created_at, updated_at) ",
        "VALUES ($1, $2, $3::jsonb, 'draft', NOW(), NOW())"
    ))
    .bind::<diesel::sql_types::Uuid, _>(task_uuid)
    .bind::<diesel::sql_types::Uuid, _>(tenant_id.into_inner())
    .bind::<diesel::sql_types::Text, _>(
        json!({
            "type": "issue",
            "issue_ref": {
                "provider": "github",
                "repository": format!("corbusier/{}", task_uuid.as_simple()),
                "issue_number": issue_number
            },
            "metadata": {
                "title": "Tenant FK test"
            }
        })
        .to_string(),
    )
    .execute(conn)
}

pub(crate) fn insert_conversation(
    conn: &mut PgConnection,
    conversation_id: ConversationId,
    tenant_id: TenantId,
    task_id: Option<TaskId>,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, tenant_id)?;
    diesel::sql_query(concat!(
        "INSERT INTO conversations (id, tenant_id, task_id, context, state, created_at, updated_at) ",
        "VALUES ($1, $2, $3, '{}'::jsonb, 'active', NOW(), NOW())"
    ))
    .bind::<diesel::sql_types::Uuid, _>(conversation_id.into_inner())
    .bind::<diesel::sql_types::Uuid, _>(tenant_id.into_inner())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(
        task_id.map(TaskId::into_inner),
    )
    .execute(conn)
}

pub(crate) fn insert_message(
    conn: &mut PgConnection,
    message_id: MessageId,
    tenant_id: TenantId,
    conversation_id: ConversationId,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, tenant_id)?;
    diesel::sql_query(concat!(
        "INSERT INTO messages (id, tenant_id, conversation_id, role, content, metadata, created_at, sequence_number) ",
        "VALUES ($1, $2, $3, 'user', $4::jsonb, '{}'::jsonb, NOW(), 1)"
    ))
    .bind::<diesel::sql_types::Uuid, _>(message_id.into_inner())
    .bind::<diesel::sql_types::Uuid, _>(tenant_id.into_inner())
    .bind::<diesel::sql_types::Uuid, _>(conversation_id.into_inner())
    .bind::<diesel::sql_types::Text, _>(json!([{"type": "text", "text": "hello"}]).to_string())
    .execute(conn)
}

pub(crate) fn insert_agent_session(
    conn: &mut PgConnection,
    params: AgentSessionInsert,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, params.tenant)?;
    let sql = if params.is_active {
        concat!(
            "INSERT INTO agent_sessions (",
            "id, tenant_id, conversation_id, agent_backend, start_sequence, end_sequence, turn_ids, ",
            "initiated_by_handoff, terminated_by_handoff, context_snapshots, started_at, ended_at, state",
            ") VALUES ($1, $2, $3, 'agent', 1, NULL, '[]'::jsonb, NULL, NULL, '[]'::jsonb, NOW(), NULL, 'active')",
        )
    } else {
        concat!(
            "INSERT INTO agent_sessions (",
            "id, tenant_id, conversation_id, agent_backend, start_sequence, end_sequence, turn_ids, ",
            "initiated_by_handoff, terminated_by_handoff, context_snapshots, started_at, ended_at, state",
            ") VALUES ($1, $2, $3, 'agent', 1, NULL, '[]'::jsonb, NULL, NULL, '[]'::jsonb, NOW(), NOW(), 'completed')",
        )
    };
    diesel::sql_query(sql)
        .bind::<diesel::sql_types::Uuid, _>(params.session.into_inner())
        .bind::<diesel::sql_types::Uuid, _>(params.tenant.into_inner())
        .bind::<diesel::sql_types::Uuid, _>(params.conversation.into_inner())
        .execute(conn)
}

pub(crate) fn insert_handoff(
    conn: &mut PgConnection,
    params: HandoffInsert,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, params.tenant)?;
    diesel::sql_query(concat!(
        "INSERT INTO handoffs (",
        "id, tenant_id, source_session_id, conversation_id, target_session_id, prior_turn_id, ",
        "triggering_tool_calls, source_agent, target_agent, reason, initiated_at, completed_at, status",
        ") VALUES ($1, $2, $3, $4, $5, $6, '[]'::jsonb, 'source', 'target', NULL, NOW(), NULL, 'initiated')"
    ))
    .bind::<diesel::sql_types::Uuid, _>(params.handoff.into_inner())
    .bind::<diesel::sql_types::Uuid, _>(params.tenant.into_inner())
    .bind::<diesel::sql_types::Uuid, _>(params.source_session.into_inner())
    .bind::<diesel::sql_types::Uuid, _>(params.conversation.into_inner())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(
        params.target_session.map(AgentSessionId::into_inner),
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .execute(conn)
}

pub(crate) fn insert_context_snapshot(
    conn: &mut PgConnection,
    params: ContextSnapshotInsert,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, params.tenant)?;
    diesel::sql_query(concat!(
        "INSERT INTO context_snapshots (",
        "id, tenant_id, conversation_id, session_id, sequence_start, sequence_end, message_summary, ",
        "visible_tool_calls, token_estimate, captured_at, snapshot_type",
        ") VALUES ($1, $2, $3, $4, 1, 1, '{}'::jsonb, '[]'::jsonb, NULL, NOW(), 'checkpoint')"
    ))
    .bind::<diesel::sql_types::Uuid, _>(params.snapshot)
    .bind::<diesel::sql_types::Uuid, _>(params.tenant.into_inner())
    .bind::<diesel::sql_types::Uuid, _>(params.conversation.into_inner())
    .bind::<diesel::sql_types::Uuid, _>(params.session.into_inner())
    .execute(conn)
}
