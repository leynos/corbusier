//! SQL helper functions for tenant schema constraint tests.
//!
//! Provides raw SQL insert functions that bypass ORM validation to test
//! database-level foreign key constraints.

use corbusier::test_support::bootstrap_tenant_row as ensure_tenant_exists;
use diesel::pg::PgConnection;
use diesel::RunQueryDsl;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub(crate) struct HandoffInsert {
    pub handoff: Uuid,
    pub tenant: Uuid,
    pub source_session: Uuid,
    pub conversation: Uuid,
}

#[derive(Clone, Copy)]
pub(crate) struct ContextSnapshotInsert {
    pub snapshot: Uuid,
    pub tenant: Uuid,
    pub conversation: Uuid,
    pub session: Uuid,
}

pub(crate) fn insert_task(
    conn: &mut PgConnection,
    task_id: Uuid,
    tenant_id: Uuid,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, tenant_id)?;
    diesel::sql_query(concat!(
        "INSERT INTO tasks (id, tenant_id, origin, state, created_at, updated_at) ",
        "VALUES ($1, $2, $3::jsonb, 'draft', NOW(), NOW())"
    ))
    .bind::<diesel::sql_types::Uuid, _>(task_id)
    .bind::<diesel::sql_types::Uuid, _>(tenant_id)
    .bind::<diesel::sql_types::Text, _>(
        json!({
            "type": "issue",
            "issue_ref": {
                "provider": "github",
                "repository": "corbusier/core",
                "issue_number": 1
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
    conversation_id: Uuid,
    tenant_id: Uuid,
    task_id: Option<Uuid>,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, tenant_id)?;
    diesel::sql_query(concat!(
        "INSERT INTO conversations (id, tenant_id, task_id, context, state, created_at, updated_at) ",
        "VALUES ($1, $2, $3, '{}'::jsonb, 'active', NOW(), NOW())"
    ))
    .bind::<diesel::sql_types::Uuid, _>(conversation_id)
    .bind::<diesel::sql_types::Uuid, _>(tenant_id)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(task_id)
    .execute(conn)
}

pub(crate) fn insert_message(
    conn: &mut PgConnection,
    message_id: Uuid,
    tenant_id: Uuid,
    conversation_id: Uuid,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, tenant_id)?;
    diesel::sql_query(concat!(
        "INSERT INTO messages (id, tenant_id, conversation_id, role, content, metadata, created_at, sequence_number) ",
        "VALUES ($1, $2, $3, 'user', $4::jsonb, '{}'::jsonb, NOW(), 1)"
    ))
    .bind::<diesel::sql_types::Uuid, _>(message_id)
    .bind::<diesel::sql_types::Uuid, _>(tenant_id)
    .bind::<diesel::sql_types::Uuid, _>(conversation_id)
    .bind::<diesel::sql_types::Text, _>(json!([{"type": "text", "text": "hello"}]).to_string())
    .execute(conn)
}

pub(crate) fn insert_agent_session(
    conn: &mut PgConnection,
    session_id: Uuid,
    tenant_id: Uuid,
    conversation_id: Uuid,
) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, tenant_id)?;
    diesel::sql_query(concat!(
        "INSERT INTO agent_sessions (",
        "id, tenant_id, conversation_id, agent_backend, start_sequence, end_sequence, turn_ids, ",
        "initiated_by_handoff, terminated_by_handoff, context_snapshots, started_at, ended_at, state",
        ") VALUES ($1, $2, $3, 'agent', 1, NULL, '[]'::jsonb, NULL, NULL, '[]'::jsonb, NOW(), NULL, 'active')"
    ))
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .bind::<diesel::sql_types::Uuid, _>(tenant_id)
    .bind::<diesel::sql_types::Uuid, _>(conversation_id)
    .execute(conn)
}

pub(crate) fn insert_handoff(conn: &mut PgConnection, params: HandoffInsert) -> diesel::QueryResult<usize> {
    ensure_tenant_exists(conn, params.tenant)?;
    diesel::sql_query(concat!(
        "INSERT INTO handoffs (",
        "id, tenant_id, source_session_id, conversation_id, target_session_id, prior_turn_id, ",
        "triggering_tool_calls, source_agent, target_agent, reason, initiated_at, completed_at, status",
        ") VALUES ($1, $2, $3, $4, NULL, $5, '[]'::jsonb, 'source', 'target', NULL, NOW(), NULL, 'initiated')"
    ))
    .bind::<diesel::sql_types::Uuid, _>(params.handoff)
    .bind::<diesel::sql_types::Uuid, _>(params.tenant)
    .bind::<diesel::sql_types::Uuid, _>(params.source_session)
    .bind::<diesel::sql_types::Uuid, _>(params.conversation)
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
    .bind::<diesel::sql_types::Uuid, _>(params.tenant)
    .bind::<diesel::sql_types::Uuid, _>(params.conversation)
    .bind::<diesel::sql_types::Uuid, _>(params.session)
    .execute(conn)
}
