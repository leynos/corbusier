//! `PostgreSQL` integration tests for tenant-aware composite foreign keys.

use diesel::Connection;
use diesel::PgConnection;
use diesel::RunQueryDsl;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, other_tenant_ctx, postgres_cluster,
    test_request_ctx,
};
use corbusier::test_support::bootstrap_tenant_row as ensure_tenant_exists;

struct TenantConstraintContext {
    temp_db: TemporaryDatabase,
}

#[derive(Clone, Copy)]
struct HandoffInsert {
    handoff: Uuid,
    tenant: Uuid,
    source_session: Uuid,
    conversation: Uuid,
}

#[derive(Clone, Copy)]
struct ContextSnapshotInsert {
    snapshot: Uuid,
    tenant: Uuid,
    conversation: Uuid,
    session: Uuid,
}

#[derive(Clone, Copy, Debug)]
enum CompositeFkCase {
    ConversationTask,
    MessageConversation,
    AgentSessionConversation,
    HandoffSourceSession,
    HandoffConversation,
    ContextSnapshotSession,
    ContextSnapshotConversation,
}

impl CompositeFkCase {
    const fn label(self) -> &'static str {
        match self {
            Self::ConversationTask => "conversations(task_id, tenant_id) -> tasks(id, tenant_id)",
            Self::MessageConversation => {
                "messages(conversation_id, tenant_id) -> conversations(id, tenant_id)"
            }
            Self::AgentSessionConversation => {
                "agent_sessions(conversation_id, tenant_id) -> conversations(id, tenant_id)"
            }
            Self::HandoffSourceSession => {
                "handoffs(source_session_id, tenant_id) -> agent_sessions(id, tenant_id)"
            }
            Self::HandoffConversation => {
                "handoffs(conversation_id, tenant_id) -> conversations(id, tenant_id)"
            }
            Self::ContextSnapshotSession => {
                "context_snapshots(session_id, tenant_id) -> agent_sessions(id, tenant_id)"
            }
            Self::ContextSnapshotConversation => {
                "context_snapshots(conversation_id, tenant_id) -> conversations(id, tenant_id)"
            }
        }
    }
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<TenantConstraintContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let temp_db = cluster
        .temporary_database_from_template(
            &format!("tenant_schema_constraints_{}", Uuid::new_v4()),
            TEMPLATE_DB,
        )
        .await?;
    Ok(TenantConstraintContext { temp_db })
}

#[rstest]
#[case(CompositeFkCase::ConversationTask)]
#[case(CompositeFkCase::MessageConversation)]
#[case(CompositeFkCase::AgentSessionConversation)]
#[case(CompositeFkCase::HandoffSourceSession)]
#[case(CompositeFkCase::HandoffConversation)]
#[case(CompositeFkCase::ContextSnapshotSession)]
#[case(CompositeFkCase::ContextSnapshotConversation)]
#[tokio::test(flavor = "multi_thread")]
async fn composite_foreign_keys_reject_cross_tenant_links(
    #[future] context: Result<TenantConstraintContext, BoxError>,
    #[case] case: CompositeFkCase,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let tenant_a = test_request_ctx();
    let tenant_b = other_tenant_ctx(&tenant_a);
    let db_url = ctx.temp_db.url().to_owned();

    let result = tokio::task::spawn_blocking(move || {
        run_fk_case(
            &db_url,
            case,
            tenant_a.tenant_id().into_inner(),
            tenant_b.tenant_id().into_inner(),
        )
    })
    .await
    .expect("spawn_blocking should not panic");

    assert!(
        result.is_err(),
        "cross-tenant insert should violate {}",
        case.label()
    );
    Ok(())
}

fn run_fk_case(
    db_url: &str,
    case: CompositeFkCase,
    tenant_a: Uuid,
    tenant_b: Uuid,
) -> Result<(), BoxError> {
    let mut conn = PgConnection::establish(db_url).map_err(|err| Box::new(err) as BoxError)?;
    conn.transaction(|tx| {
        execute_fk_case(tx, case, tenant_a, tenant_b)
            .map(|_| ())
            .map_err(|err| Box::new(err) as BoxError)
    })
}

fn execute_fk_case(
    tx: &mut PgConnection,
    case: CompositeFkCase,
    tenant_a: Uuid,
    tenant_b: Uuid,
) -> diesel::QueryResult<usize> {
    match case {
        CompositeFkCase::ConversationTask => {
            let task_id = Uuid::new_v4();
            insert_task(tx, task_id, tenant_a)?;
            insert_conversation(tx, Uuid::new_v4(), tenant_b, Some(task_id))
        }
        CompositeFkCase::MessageConversation => {
            let conversation_id = Uuid::new_v4();
            insert_conversation(tx, conversation_id, tenant_a, None)?;
            insert_message(tx, Uuid::new_v4(), tenant_b, conversation_id)
        }
        CompositeFkCase::AgentSessionConversation => {
            let conversation_id = Uuid::new_v4();
            insert_conversation(tx, conversation_id, tenant_a, None)?;
            insert_agent_session(tx, Uuid::new_v4(), tenant_b, conversation_id)
        }
        CompositeFkCase::HandoffSourceSession => {
            insert_handoff_source_session_case(tx, tenant_a, tenant_b)
        }
        CompositeFkCase::HandoffConversation => {
            insert_handoff_conversation_case(tx, tenant_a, tenant_b)
        }
        CompositeFkCase::ContextSnapshotSession => {
            insert_context_snapshot_session_case(tx, tenant_a, tenant_b)
        }
        CompositeFkCase::ContextSnapshotConversation => {
            insert_context_snapshot_conversation_case(tx, tenant_a, tenant_b)
        }
    }
}

fn insert_handoff_source_session_case(
    tx: &mut PgConnection,
    tenant_a: Uuid,
    tenant_b: Uuid,
) -> diesel::QueryResult<usize> {
    let (session, conversation) =
        build_cross_tenant_prerequisites(tx, tenant_a, tenant_b, CrossTenantField::Session)?;
    insert_handoff(
        tx,
        HandoffInsert {
            handoff: Uuid::new_v4(),
            tenant: tenant_b,
            source_session: session,
            conversation,
        },
    )
}

fn insert_handoff_conversation_case(
    tx: &mut PgConnection,
    tenant_a: Uuid,
    tenant_b: Uuid,
) -> diesel::QueryResult<usize> {
    let (session, conversation) =
        build_cross_tenant_prerequisites(tx, tenant_a, tenant_b, CrossTenantField::Conversation)?;
    insert_handoff(
        tx,
        HandoffInsert {
            handoff: Uuid::new_v4(),
            tenant: tenant_b,
            source_session: session,
            conversation,
        },
    )
}

#[derive(Clone, Copy)]
enum CrossTenantField {
    Session,
    Conversation,
}

fn build_cross_tenant_prerequisites(
    tx: &mut PgConnection,
    tenant_a: Uuid,
    tenant_b: Uuid,
    mismatched: CrossTenantField,
) -> diesel::QueryResult<(Uuid, Uuid)> {
    let conversation_a = Uuid::new_v4();
    let conversation_b = Uuid::new_v4();
    insert_conversation(tx, conversation_a, tenant_a, None)?;
    insert_conversation(tx, conversation_b, tenant_b, None)?;

    match mismatched {
        CrossTenantField::Session => {
            let session_a = Uuid::new_v4();
            insert_agent_session(tx, session_a, tenant_a, conversation_a)?;
            Ok((session_a, conversation_b))
        }
        CrossTenantField::Conversation => {
            let session_b = Uuid::new_v4();
            insert_agent_session(tx, session_b, tenant_b, conversation_b)?;
            Ok((session_b, conversation_a))
        }
    }
}

fn insert_context_snapshot_session_case(
    tx: &mut PgConnection,
    tenant_a: Uuid,
    tenant_b: Uuid,
) -> diesel::QueryResult<usize> {
    let (session, conversation) =
        build_cross_tenant_prerequisites(tx, tenant_a, tenant_b, CrossTenantField::Session)?;
    insert_context_snapshot(
        tx,
        ContextSnapshotInsert {
            snapshot: Uuid::new_v4(),
            tenant: tenant_b,
            conversation,
            session,
        },
    )
}

fn insert_context_snapshot_conversation_case(
    tx: &mut PgConnection,
    tenant_a: Uuid,
    tenant_b: Uuid,
) -> diesel::QueryResult<usize> {
    let (session, conversation) =
        build_cross_tenant_prerequisites(tx, tenant_a, tenant_b, CrossTenantField::Conversation)?;
    insert_context_snapshot(
        tx,
        ContextSnapshotInsert {
            snapshot: Uuid::new_v4(),
            tenant: tenant_b,
            conversation,
            session,
        },
    )
}

fn insert_task(
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

fn insert_conversation(
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

fn insert_message(
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

fn insert_agent_session(
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

fn insert_handoff(conn: &mut PgConnection, params: HandoffInsert) -> diesel::QueryResult<usize> {
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

fn insert_context_snapshot(
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
