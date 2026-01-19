//! Integration tests for internal SQL helper functions.
//!
//! These tests require a running PostgreSQL instance and exercise the SQL
//! helpers through the repository stack rather than in isolation.

use corbusier::message::{
    domain::{ContentPart, ConversationId, Message, MessageId, Role, SequenceNumber, TextPart},
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;
use uuid::Uuid;

use super::helpers::{
    CleanupGuard, ExpectedAuditContext, PostgresCluster, clock, ensure_template,
    fetch_audit_log_for_message, insert_conversation, postgres_cluster, setup_repository,
    test_runtime,
};

// ============================================================================
// Constraint Error Mapping Tests
// ============================================================================

/// Tests that inserting a message with duplicate ID returns `DuplicateMessage` error.
#[rstest]
fn insert_message_maps_duplicate_id_constraint(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
) {
    let db_name = format!("sql_helpers_dup_id_{}", Uuid::new_v4());
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, &db_name, conv_id).expect("conversation insert");

    let msg_id = MessageId::new();
    let message1 = Message::new_with_id(
        msg_id,
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("First message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let message2 = Message::new_with_id(
        msg_id, // Same ID
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Second message"))],
        SequenceNumber::new(2), // Different sequence
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime().expect("tokio runtime");

    // Store first message
    rt.block_on(repo.store(&message1)).expect("first store");

    // Second store should fail with DuplicateMessage
    let result = rt.block_on(repo.store(&message2));
    match result {
        Err(RepositoryError::DuplicateMessage(id)) => {
            assert_eq!(id, msg_id, "error should contain the duplicate message ID");
        }
        other => panic!("expected DuplicateMessage error, got {other:?}"),
    }

    drop(repo);

    guard.cleanup().expect("cleanup database");
}

/// Tests that inserting a message with duplicate sequence returns `DuplicateSequence` error.
#[rstest]
fn insert_message_maps_duplicate_sequence_constraint(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
) {
    let db_name = format!("sql_helpers_dup_seq_{}", Uuid::new_v4());
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, &db_name, conv_id).expect("conversation insert");

    let message1 = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("First message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let message2 = Message::new(
        conv_id, // Same conversation
        Role::User,
        vec![ContentPart::Text(TextPart::new("Second message"))],
        SequenceNumber::new(1), // Same sequence number
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime().expect("tokio runtime");

    // Store first message
    rt.block_on(repo.store(&message1)).expect("first store");

    // Second store should fail with DuplicateSequence
    let result = rt.block_on(repo.store(&message2));
    match result {
        Err(RepositoryError::DuplicateSequence {
            conversation_id,
            sequence,
        }) => {
            assert_eq!(conversation_id, conv_id);
            assert_eq!(sequence.value(), 1);
        }
        other => panic!("expected DuplicateSequence error, got {other:?}"),
    }

    drop(repo);

    guard.cleanup().expect("cleanup database");
}

// ============================================================================
// Audit Context Setting Tests
// ============================================================================

/// Tests that `set_audit_context` correctly sets `PostgreSQL` session variables.
///
/// Parameterized across three scenarios:
/// - Full context: all fields populated
/// - Empty context: all fields `None`
/// - Partial context: only `correlation_id` and `user_id` populated
#[rstest]
#[case::full_context(
    ExpectedAuditContext {
        correlation_id: Some(Uuid::new_v4()),
        causation_id: Some(Uuid::new_v4()),
        user_id: Some(Uuid::new_v4()),
        session_id: Some(Uuid::new_v4()),
    },
    "full"
)]
#[case::empty_context(
    ExpectedAuditContext {
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    },
    "empty"
)]
#[case::partial_context(
    ExpectedAuditContext {
        correlation_id: Some(Uuid::new_v4()),
        causation_id: None,
        user_id: Some(Uuid::new_v4()),
        session_id: None,
    },
    "partial"
)]
fn set_audit_context_propagates_fields(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
    #[case] expected: ExpectedAuditContext,
    #[case] scenario: &str,
) {
    let db_name = format!("sql_helpers_audit_{scenario}_{}", Uuid::new_v4());
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, &db_name, conv_id).expect("conversation insert");

    let audit = expected.to_audit_context();

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audit test message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime().expect("tokio runtime");
    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store with audit");

    // Verify via audit log
    let audit_log = fetch_audit_log_for_message(cluster, &db_name, message.id().into_inner())
        .expect("fetch audit log")
        .expect("audit log should exist");

    assert_eq!(audit_log.correlation_id, expected.correlation_id);
    assert_eq!(audit_log.causation_id, expected.causation_id);
    assert_eq!(audit_log.user_id, expected.user_id);
    assert_eq!(audit_log.session_id, expected.session_id);

    drop(repo);

    guard.cleanup().expect("cleanup database");
}

// ============================================================================
// Insert Message Tests
// ============================================================================

/// Tests that `insert_message` successfully inserts a valid message.
#[rstest]
fn insert_message_succeeds_for_valid_message(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
) {
    let db_name = format!("sql_helpers_insert_valid_{}", Uuid::new_v4());
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, &db_name, conv_id).expect("conversation insert");

    let message = Message::new(
        conv_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Hello from assistant"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime().expect("tokio runtime");
    rt.block_on(repo.store(&message))
        .expect("store should succeed");

    // Verify the message was stored
    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find should succeed")
        .expect("message should exist");

    assert_eq!(retrieved.id(), message.id());
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::Assistant);

    drop(repo);

    guard.cleanup().expect("cleanup database");
}

/// Tests that generic database errors (not constraint violations) are wrapped correctly.
#[rstest]
fn insert_message_wraps_generic_database_errors(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
) {
    let db_name = format!("sql_helpers_insert_fk_{}", Uuid::new_v4());
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repo");

    // Don't insert the conversation - this will trigger a foreign key violation
    let conv_id = ConversationId::new();

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Orphan message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime().expect("tokio runtime");
    let result = rt.block_on(repo.store(&message));

    // Should get a Database error (not DuplicateMessage or DuplicateSequence)
    match result {
        Err(RepositoryError::Database(_)) => {
            // Expected - foreign key violation is wrapped as Database error
        }
        other => panic!("expected Database error for FK violation, got {other:?}"),
    }

    drop(repo);

    guard.cleanup().expect("cleanup database");
}
