//! Audit context tests for `PostgreSQL` message repository.
//!
//! Tests `store_with_audit` session variable propagation and `audit_logs` table
//! verification via the database trigger.

use crate::postgres::helpers::{
    CleanupGuard, clock, ensure_template, fetch_audit_log_for_message, insert_conversation,
    setup_repository, test_runtime,
};
use corbusier::message::{
    adapters::audit_context::AuditContext,
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use pg_embedded_setup_unpriv::TestCluster;
use pg_embedded_setup_unpriv::test_support::shared_test_cluster;
use rstest::rstest;
use uuid::Uuid;

/// Expected audit context values for parameterized tests.
struct ExpectedAuditContext {
    correlation: Option<Uuid>,
    causation: Option<Uuid>,
    user: Option<Uuid>,
    session: Option<Uuid>,
}

/// Creates an `AuditContext` from expected values.
const fn create_audit_context(expected: &ExpectedAuditContext) -> AuditContext {
    let mut audit = AuditContext::empty();
    if let Some(id) = expected.correlation {
        audit = audit.with_correlation_id(id);
    }
    if let Some(id) = expected.causation {
        audit = audit.with_causation_id(id);
    }
    if let Some(id) = expected.user {
        audit = audit.with_user_id(id);
    }
    if let Some(id) = expected.session {
        audit = audit.with_session_id(id);
    }
    audit
}

/// Tests `store_with_audit` correctly propagates audit context to `audit_logs` table.
///
/// Parameterized across three scenarios:
/// - Full context: all fields populated
/// - Empty context: all fields None
/// - Partial context: only `correlation_id` populated
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
    shared_test_cluster: &'static TestCluster,
    #[case] expected: ExpectedAuditContext,
    #[case] scenario: &str,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_audit_{scenario}_{}", Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audited message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let audit = create_audit_context(&expected);

    let rt = test_runtime();

    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store_with_audit");

    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find")
        .expect("exists");

    assert_eq!(retrieved.id(), message.id());

    // Verify audit_logs entry was created with correct context
    let audit_log =
        fetch_audit_log_for_message(shared_test_cluster, &db_name, message.id().into_inner())
            .expect("audit log query should succeed")
            .expect("audit log entry should exist");

    assert_eq!(audit_log.table_name, "messages");
    assert_eq!(audit_log.operation, "INSERT");
    assert_eq!(audit_log.row_id, Some(message.id().into_inner()));
    assert_eq!(audit_log.correlation_id, expected.correlation);
    assert_eq!(audit_log.causation_id, expected.causation);
    assert_eq!(audit_log.user_id, expected.user);
    assert_eq!(audit_log.session_id, expected.session);
}
