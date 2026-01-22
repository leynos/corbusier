//! Integration tests for internal SQL helper functions.
//!
//! These tests require a running `PostgreSQL` instance and exercise the SQL
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
    ExpectedAuditContext, PostgresCluster, clock, ensure_template, fetch_audit_log_for_message,
    insert_conversation, postgres_cluster, setup_repository,
};

// ============================================================================
// Constraint Error Mapping Tests
// ============================================================================

/// Tests that inserting a message with duplicate ID returns `DuplicateMessage` error.
#[rstest]
#[tokio::test]
async fn insert_message_maps_duplicate_id_constraint(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (temp_db, repo) = setup_repository(cluster).await.expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id)
        .await
        .expect("conversation insert");

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

    // Store first message
    repo.store(&message1).await.expect("first store");

    // Second store should fail with DuplicateMessage
    let result = repo.store(&message2).await;
    match result {
        Err(RepositoryError::DuplicateMessage(id)) => {
            assert_eq!(id, msg_id, "error should contain the duplicate message ID");
        }
        other => panic!("expected DuplicateMessage error, got {other:?}"),
    }
}

/// Tests that inserting a message with duplicate sequence returns `DuplicateSequence` error.
#[rstest]
#[tokio::test]
async fn insert_message_maps_duplicate_sequence_constraint(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (temp_db, repo) = setup_repository(cluster).await.expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id)
        .await
        .expect("conversation insert");

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

    // Store first message
    repo.store(&message1).await.expect("first store");

    // Second store should fail with DuplicateSequence
    let result = repo.store(&message2).await;
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
}

// ============================================================================
// Audit Context Setting Tests
// ============================================================================

/// Tests that `set_audit_context` correctly sets `PostgreSQL` session variables.
///
/// Parameterized across three scenarios:
/// - Full context: all fields populated
/// - Empty context: all fields `None`
/// - Partial context: only `correlation` and `user` populated
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
        user: Some(Uuid::new_v4()),
        session: None,
    },
    "partial"
)]
#[tokio::test]
#[expect(
    clippy::used_underscore_binding,
    reason = "Scenario parameter required for test case naming but not used in test body"
)]
async fn set_audit_context_propagates_fields(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
    #[case] expected: ExpectedAuditContext,
    #[case] _scenario: &str,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (temp_db, repo) = setup_repository(cluster).await.expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id)
        .await
        .expect("conversation insert");

    let audit = expected.to_audit_context();

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audit test message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    repo.store_with_audit(&message, &audit)
        .await
        .expect("store with audit");

    // Verify via audit log
    let audit_log = fetch_audit_log_for_message(cluster, temp_db.name(), message.id().into_inner())
        .await
        .expect("fetch audit log")
        .expect("audit log should exist");

    assert_eq!(audit_log.correlation_id, expected.correlation);
    assert_eq!(audit_log.causation_id, expected.causation);
    assert_eq!(audit_log.user_id, expected.user);
    assert_eq!(audit_log.session_id, expected.session);
}

// ============================================================================
// Insert Message Tests
// ============================================================================

/// Tests that `insert_message` successfully inserts a valid message.
#[rstest]
#[tokio::test]
async fn insert_message_succeeds_for_valid_message(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (temp_db, repo) = setup_repository(cluster).await.expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id)
        .await
        .expect("conversation insert");

    let message = Message::new(
        conv_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Hello from assistant"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    repo.store(&message).await.expect("store should succeed");

    // Verify the message was stored
    let retrieved = repo
        .find_by_id(message.id())
        .await
        .expect("find should succeed")
        .expect("message should exist");

    assert_eq!(retrieved.id(), message.id());
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::Assistant);
}

/// Tests that generic database errors (not constraint violations) are wrapped correctly.
#[rstest]
#[tokio::test]
#[expect(
    clippy::used_underscore_binding,
    reason = "Database and repo kept alive via RAII but not explicitly used"
)]
async fn insert_message_wraps_generic_database_errors(
    postgres_cluster: PostgresCluster,
    clock: DefaultClock,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (_temp_db, _repo) = setup_repository(cluster).await.expect("repo");

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

    let result = _repo.store(&message).await;

    // Should get a Database error (not DuplicateMessage or DuplicateSequence)
    match result {
        Err(RepositoryError::Database(_)) => {
            // Expected - foreign key violation is wrapped as Database error
        }
        other => panic!("expected Database error for FK violation, got {other:?}"),
    }
}
