//! Audit context tests for `PostgreSQL` message repository.
//!
//! Tests `store_with_audit` session variable propagation and `audit_logs` table
//! verification via the database trigger.

use crate::postgres::helpers::{
    CleanupGuard, ExpectedAuditContext, PostgresCluster, clock, ensure_template,
    fetch_audit_log_for_message, insert_conversation, postgres_cluster, setup_repository,
    test_runtime,
};
use corbusier::message::{
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;
use uuid::Uuid;

/// Tests `store_with_audit` correctly propagates audit context to `audit_logs` table.
///
/// Parameterized across three scenarios:
/// - Full context: all fields populated
/// - Empty context: all fields None
/// - Partial context: only `correlation` populated
#[rstest]
#[case::full_context(
    ExpectedAuditContext {
        correlation: Some(Uuid::new_v4()),
        causation: Some(Uuid::new_v4()),
        user: Some(Uuid::new_v4()),
        session: Some(Uuid::new_v4()),
    },
    "full"
)]
#[case::empty_context(
    ExpectedAuditContext {
        correlation: None,
        causation: None,
        user: None,
        session: None,
    },
    "empty"
)]
#[case::partial_context(
    ExpectedAuditContext {
        correlation: Some(Uuid::new_v4()),
        causation: None,
        user: None,
        session: None,
    },
    "partial"
)]
fn store_with_audit_captures_context(
    clock: DefaultClock,
    postgres_cluster: PostgresCluster,
    #[case] expected: ExpectedAuditContext,
    #[case] scenario: &str,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let db_name = format!("test_audit_{scenario}_{}", Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, &db_name, conv_id).expect("conversation insert");

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audited message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let audit = expected.to_audit_context();

    let rt = test_runtime().expect("tokio runtime");

    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store_with_audit");

    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find")
        .expect("message should exist");

    assert_eq!(retrieved.id(), message.id());

    // Verify audit_logs entry was created with correct context
    let audit_log = fetch_audit_log_for_message(cluster, &db_name, message.id().into_inner())
        .expect("audit log query should succeed")
        .expect("audit log entry should exist");

    assert_eq!(audit_log.table_name, "messages");
    assert_eq!(audit_log.operation, "INSERT");
    assert_eq!(audit_log.row_id, Some(message.id().into_inner()));
    assert_eq!(audit_log.correlation_id, expected.correlation);
    assert_eq!(audit_log.causation_id, expected.causation);
    assert_eq!(audit_log.user_id, expected.user);
    assert_eq!(audit_log.session_id, expected.session);

    drop(repo);

    guard.cleanup().expect("cleanup database");
}
