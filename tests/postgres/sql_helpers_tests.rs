//! Unit tests for internal SQL helper functions.
//!
//! Tests the constraint error mapping and audit context setting functions
//! in isolation from the full repository operations.

use corbusier::message::{
    adapters::audit_context::AuditContext,
    domain::{ContentPart, ConversationId, Message, MessageId, Role, SequenceNumber, TextPart},
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use pg_embedded_setup_unpriv::{TestCluster, test_support::shared_test_cluster};
use rstest::rstest;
use uuid::Uuid;

use super::helpers::{
    CleanupGuard, clock, ensure_template, fetch_audit_log_for_message, insert_conversation,
    setup_repository, test_runtime,
};

// ============================================================================
// Constraint Error Mapping Tests
// ============================================================================

/// Tests that inserting a message with duplicate ID returns `DuplicateMessage` error.
#[rstest]
fn insert_message_maps_duplicate_id_constraint(
    shared_test_cluster: &'static TestCluster,
    clock: DefaultClock,
) {
    let db_name = "sql_helpers_dup_id";
    ensure_template(shared_test_cluster).expect("template");
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.to_owned());
    let repo = setup_repository(shared_test_cluster, db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, db_name, conv_id);

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

    let rt = test_runtime();

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
}

/// Tests that inserting a message with duplicate sequence returns `DuplicateSequence` error.
#[rstest]
fn insert_message_maps_duplicate_sequence_constraint(
    shared_test_cluster: &'static TestCluster,
    clock: DefaultClock,
) {
    let db_name = "sql_helpers_dup_seq";
    ensure_template(shared_test_cluster).expect("template");
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.to_owned());
    let repo = setup_repository(shared_test_cluster, db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, db_name, conv_id);

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

    let rt = test_runtime();

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
}

// ============================================================================
// Audit Context Setting Tests
// ============================================================================

/// Tests that `set_audit_context` sets all provided fields in `PostgreSQL` session variables.
#[rstest]
fn set_audit_context_sets_all_fields(
    shared_test_cluster: &'static TestCluster,
    clock: DefaultClock,
) {
    let db_name = "sql_helpers_audit_all";
    ensure_template(shared_test_cluster).expect("template");
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.to_owned());
    let repo = setup_repository(shared_test_cluster, db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, db_name, conv_id);

    let correlation_id = Uuid::new_v4();
    let causation_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    let audit = AuditContext::empty()
        .with_correlation_id(correlation_id)
        .with_causation_id(causation_id)
        .with_user_id(user_id)
        .with_session_id(session_id);

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audit test message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store with audit");

    // Verify via audit log
    let audit_log =
        fetch_audit_log_for_message(shared_test_cluster, db_name, message.id().into_inner())
            .expect("fetch audit log")
            .expect("audit log should exist");

    assert_eq!(audit_log.correlation_id, Some(correlation_id));
    assert_eq!(audit_log.causation_id, Some(causation_id));
    assert_eq!(audit_log.user_id, Some(user_id));
    assert_eq!(audit_log.session_id, Some(session_id));
}

/// Tests that `set_audit_context` handles empty audit context (no fields set).
#[rstest]
fn set_audit_context_handles_empty_context(
    shared_test_cluster: &'static TestCluster,
    clock: DefaultClock,
) {
    let db_name = "sql_helpers_audit_empty";
    ensure_template(shared_test_cluster).expect("template");
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.to_owned());
    let repo = setup_repository(shared_test_cluster, db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, db_name, conv_id);

    let audit = AuditContext::empty();

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Empty audit test"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store with empty audit");

    // Verify via audit log - all audit fields should be None
    let audit_log =
        fetch_audit_log_for_message(shared_test_cluster, db_name, message.id().into_inner())
            .expect("fetch audit log")
            .expect("audit log should exist");

    assert_eq!(audit_log.correlation_id, None);
    assert_eq!(audit_log.causation_id, None);
    assert_eq!(audit_log.user_id, None);
    assert_eq!(audit_log.session_id, None);
}

/// Tests that `set_audit_context` handles partial audit context (some fields set).
#[rstest]
fn set_audit_context_handles_partial_context(
    shared_test_cluster: &'static TestCluster,
    clock: DefaultClock,
) {
    let db_name = "sql_helpers_audit_partial";
    ensure_template(shared_test_cluster).expect("template");
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.to_owned());
    let repo = setup_repository(shared_test_cluster, db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, db_name, conv_id);

    let correlation_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Only set correlation_id and user_id
    let audit = AuditContext::empty()
        .with_correlation_id(correlation_id)
        .with_user_id(user_id);

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Partial audit test"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store with partial audit");

    // Verify via audit log
    let audit_log =
        fetch_audit_log_for_message(shared_test_cluster, db_name, message.id().into_inner())
            .expect("fetch audit log")
            .expect("audit log should exist");

    assert_eq!(audit_log.correlation_id, Some(correlation_id));
    assert_eq!(audit_log.causation_id, None);
    assert_eq!(audit_log.user_id, Some(user_id));
    assert_eq!(audit_log.session_id, None);
}

// ============================================================================
// Insert Message Tests
// ============================================================================

/// Tests that `insert_message` successfully inserts a valid message.
#[rstest]
fn insert_message_succeeds_for_valid_message(
    shared_test_cluster: &'static TestCluster,
    clock: DefaultClock,
) {
    let db_name = "sql_helpers_insert_valid";
    ensure_template(shared_test_cluster).expect("template");
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.to_owned());
    let repo = setup_repository(shared_test_cluster, db_name).expect("repo");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, db_name, conv_id);

    let message = Message::new(
        conv_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Hello from assistant"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime();
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
}

/// Tests that generic database errors (not constraint violations) are wrapped correctly.
#[rstest]
fn insert_message_wraps_generic_database_errors(
    shared_test_cluster: &'static TestCluster,
    clock: DefaultClock,
) {
    let db_name = "sql_helpers_insert_fk";
    ensure_template(shared_test_cluster).expect("template");
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.to_owned());
    let repo = setup_repository(shared_test_cluster, db_name).expect("repo");

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

    let rt = test_runtime();
    let result = rt.block_on(repo.store(&message));

    // Should get a Database error (not DuplicateMessage or DuplicateSequence)
    match result {
        Err(RepositoryError::Database(_)) => {
            // Expected - foreign key violation is wrapped as Database error
        }
        other => panic!("expected Database error for FK violation, got {other:?}"),
    }
}
