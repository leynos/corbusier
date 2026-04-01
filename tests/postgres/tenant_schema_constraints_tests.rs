//! `PostgreSQL` integration tests for tenant-aware composite foreign keys.

use corbusier::context::TenantId;
use corbusier::message::domain::{AgentSessionId, ConversationId, HandoffId, MessageId};
use corbusier::task::domain::TaskId;
use diesel::Connection;
use diesel::PgConnection;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use rstest::{fixture, rstest};
use uuid::Uuid;

#[path = "tenant_schema_constraints_helpers.rs"]
mod helpers;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, other_tenant_ctx, postgres_cluster,
    test_request_ctx,
};
use helpers::{
    AgentSessionInsert, ContextSnapshotId, ContextSnapshotInsert, HandoffInsert,
    insert_agent_session, insert_context_snapshot, insert_conversation, insert_handoff,
    insert_message, insert_task,
};

struct TenantConstraintContext {
    temp_db: TemporaryDatabase,
}

#[derive(Clone, Copy, Debug)]
enum CompositeFkCase {
    ConversationTask,
    MessageConversation,
    AgentSessionConversation,
    HandoffSourceSession,
    HandoffConversation,
    HandoffTargetSession,
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
            Self::HandoffTargetSession => {
                "handoffs(target_session_id, tenant_id) -> agent_sessions(id, tenant_id)"
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
#[case(CompositeFkCase::HandoffTargetSession)]
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

    // We expect a foreign key violation (SQLSTATE 23503), not just any error.
    let err = result.expect_err(&format!(
        "cross-tenant insert should violate {}, but succeeded",
        case.label()
    ));

    let diesel_err = err
        .downcast_ref::<DieselError>()
        .expect("expected diesel::result::Error from run_fk_case");

    match diesel_err {
        DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => {
            // Expected path: cross-tenant FK insert is rejected.
        }
        other => panic!(
            "expected ForeignKeyViolation (SQLSTATE 23503) for {}, got: {:?}",
            case.label(),
            other
        ),
    }

    Ok(())
}

#[rstest]
#[case(CompositeFkCase::ConversationTask)]
#[case(CompositeFkCase::MessageConversation)]
#[case(CompositeFkCase::AgentSessionConversation)]
#[case(CompositeFkCase::HandoffSourceSession)]
#[case(CompositeFkCase::HandoffConversation)]
#[case(CompositeFkCase::HandoffTargetSession)]
#[case(CompositeFkCase::ContextSnapshotSession)]
#[case(CompositeFkCase::ContextSnapshotConversation)]
#[tokio::test(flavor = "multi_thread")]
async fn composite_foreign_keys_accept_same_tenant_links(
    #[future] context: Result<TenantConstraintContext, BoxError>,
    #[case] case: CompositeFkCase,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let tenant = test_request_ctx();
    let db_url = ctx.temp_db.url().to_owned();

    let result = tokio::task::spawn_blocking(move || {
        run_fk_case(
            &db_url,
            case,
            tenant.tenant_id().into_inner(),
            tenant.tenant_id().into_inner(),
        )
    })
    .await
    .expect("spawn_blocking should not panic");

    assert!(
        result.is_ok(),
        "same-tenant insert should succeed for {}",
        case.label()
    );
    Ok(())
}

fn run_fk_case(
    db_url: &str,
    case: CompositeFkCase,
    first_tenant_uuid: Uuid,
    second_tenant_uuid: Uuid,
) -> Result<(), BoxError> {
    let mut conn = PgConnection::establish(db_url).map_err(|err| Box::new(err) as BoxError)?;
    let tenant_a = TenantId::from_uuid(first_tenant_uuid);
    let tenant_b = TenantId::from_uuid(second_tenant_uuid);
    conn.transaction(|tx| {
        execute_fk_case(tx, case, tenant_a, tenant_b)
            .map(|_| ())
            .map_err(|err| Box::new(err) as BoxError)
    })
}

fn execute_fk_case(
    tx: &mut PgConnection,
    case: CompositeFkCase,
    tenant_a: TenantId,
    tenant_b: TenantId,
) -> diesel::QueryResult<usize> {
    match case {
        CompositeFkCase::ConversationTask => {
            let task_id = TaskId::new();
            insert_task(tx, task_id, tenant_a)?;
            insert_conversation(tx, ConversationId::new(), tenant_b, Some(task_id))
        }
        CompositeFkCase::MessageConversation => {
            let conversation_id = ConversationId::new();
            insert_conversation(tx, conversation_id, tenant_a, None)?;
            insert_message(tx, MessageId::new(), tenant_b, conversation_id)
        }
        CompositeFkCase::AgentSessionConversation => {
            let conversation_id = ConversationId::new();
            insert_conversation(tx, conversation_id, tenant_a, None)?;
            insert_agent_session(
                tx,
                AgentSessionInsert {
                    session: AgentSessionId::new(),
                    tenant: tenant_b,
                    conversation: conversation_id,
                    is_active: true,
                },
            )
        }
        CompositeFkCase::HandoffSourceSession => {
            insert_handoff_source_session_case(tx, tenant_a, tenant_b)
        }
        CompositeFkCase::HandoffConversation => {
            insert_handoff_conversation_case(tx, tenant_a, tenant_b)
        }
        CompositeFkCase::HandoffTargetSession => {
            insert_handoff_target_session_case(tx, tenant_a, tenant_b)
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
    tenant_a: TenantId,
    tenant_b: TenantId,
) -> diesel::QueryResult<usize> {
    let (session, conversation) =
        build_cross_tenant_prerequisites(tx, tenant_a, tenant_b, CrossTenantField::Session)?;
    insert_handoff(
        tx,
        HandoffInsert {
            handoff: HandoffId::new(),
            tenant: tenant_b,
            source_session: session,
            conversation,
            target_session: None,
        },
    )
}

fn insert_handoff_conversation_case(
    tx: &mut PgConnection,
    tenant_a: TenantId,
    tenant_b: TenantId,
) -> diesel::QueryResult<usize> {
    let (session, conversation) =
        build_cross_tenant_prerequisites(tx, tenant_a, tenant_b, CrossTenantField::Conversation)?;
    insert_handoff(
        tx,
        HandoffInsert {
            handoff: HandoffId::new(),
            tenant: tenant_b,
            source_session: session,
            conversation,
            target_session: None,
        },
    )
}

fn insert_handoff_target_session_case(
    tx: &mut PgConnection,
    tenant_a: TenantId,
    tenant_b: TenantId,
) -> diesel::QueryResult<usize> {
    let conversation_b = ConversationId::new();
    insert_conversation(tx, conversation_b, tenant_b, None)?;
    let source_session_b = AgentSessionId::new();
    insert_agent_session(
        tx,
        AgentSessionInsert {
            session: source_session_b,
            tenant: tenant_b,
            conversation: conversation_b,
            is_active: true,
        },
    )?;

    let conversation_a = ConversationId::new();
    insert_conversation(tx, conversation_a, tenant_a, None)?;
    let target_session_a = AgentSessionId::new();
    insert_agent_session(
        tx,
        AgentSessionInsert {
            session: target_session_a,
            tenant: tenant_a,
            conversation: conversation_a,
            is_active: true,
        },
    )?;

    insert_handoff(
        tx,
        HandoffInsert {
            handoff: HandoffId::new(),
            tenant: tenant_b,
            source_session: source_session_b,
            conversation: conversation_b,
            target_session: Some(target_session_a),
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
    tenant_a: TenantId,
    tenant_b: TenantId,
    mismatched: CrossTenantField,
) -> diesel::QueryResult<(AgentSessionId, ConversationId)> {
    let conversation_a = ConversationId::new();
    let conversation_b = ConversationId::new();
    insert_conversation(tx, conversation_a, tenant_a, None)?;
    insert_conversation(tx, conversation_b, tenant_b, None)?;

    match mismatched {
        CrossTenantField::Session => {
            let session_a = AgentSessionId::new();
            insert_agent_session(
                tx,
                AgentSessionInsert {
                    session: session_a,
                    tenant: tenant_a,
                    conversation: conversation_a,
                    is_active: true,
                },
            )?;
            Ok((session_a, conversation_b))
        }
        CrossTenantField::Conversation => {
            let session_b = AgentSessionId::new();
            insert_agent_session(
                tx,
                AgentSessionInsert {
                    session: session_b,
                    tenant: tenant_b,
                    conversation: conversation_b,
                    is_active: true,
                },
            )?;
            Ok((session_b, conversation_a))
        }
    }
}

fn insert_context_snapshot_session_case(
    tx: &mut PgConnection,
    tenant_a: TenantId,
    tenant_b: TenantId,
) -> diesel::QueryResult<usize> {
    let (session, conversation) =
        build_cross_tenant_prerequisites(tx, tenant_a, tenant_b, CrossTenantField::Session)?;
    insert_context_snapshot(
        tx,
        ContextSnapshotInsert {
            snapshot: ContextSnapshotId::default(),
            tenant: tenant_b,
            conversation,
            session,
        },
    )
}

fn insert_context_snapshot_conversation_case(
    tx: &mut PgConnection,
    tenant_a: TenantId,
    tenant_b: TenantId,
) -> diesel::QueryResult<usize> {
    let (session, conversation) =
        build_cross_tenant_prerequisites(tx, tenant_a, tenant_b, CrossTenantField::Conversation)?;
    insert_context_snapshot(
        tx,
        ContextSnapshotInsert {
            snapshot: ContextSnapshotId::default(),
            tenant: tenant_b,
            conversation,
            session,
        },
    )
}
