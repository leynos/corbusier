//! Integration tests for internal SQL helper functions.
//!
//! These tests require a running `PostgreSQL` instance and exercise the SQL
//! helpers through the repository stack rather than in isolation.

#![expect(
    clippy::too_many_arguments,
    reason = "rstest fixture injection adds parameters beyond the Clippy threshold"
)]

use corbusier::context::{CausationId, RequestContext};
use corbusier::message::{
    domain::{ContentPart, ConversationId, Message, MessageId, Role, SequenceNumber, TextPart},
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;

use super::cluster::BoxError;
use super::helpers::{
    PreparedRepo, clock, fetch_audit_log_for_message, insert_conversation, prepared_repo,
    test_request_context,
};

// ============================================================================
// Constraint Error Mapping Tests
// ============================================================================

/// Tests that inserting a message with duplicate ID returns `DuplicateMessage` error.
#[rstest]
#[tokio::test]
async fn insert_message_maps_duplicate_id_constraint(
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
    clock: DefaultClock,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let ctx = prepared_repo.await?;

    let conv_id = ConversationId::new();
    insert_conversation(ctx.cluster, ctx.temp_db.name(), conv_id).await?;

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

    let req_ctx = test_request_context;

    // Store first message
    ctx.repo.store(&req_ctx, &message1).await?;

    // Second store should fail with DuplicateMessage
    let result = ctx.repo.store(&req_ctx, &message2).await;
    match result {
        Err(RepositoryError::DuplicateMessage(id)) => {
            assert_eq!(id, msg_id, "error should contain the duplicate message ID");
        }
        other => panic!("expected DuplicateMessage error, got {other:?}"),
    }
    Ok(())
}

/// Tests that inserting a message with duplicate sequence returns `DuplicateSequence` error.
#[rstest]
#[tokio::test]
async fn insert_message_maps_duplicate_sequence_constraint(
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
    clock: DefaultClock,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let ctx = prepared_repo.await?;

    let conv_id = ConversationId::new();
    insert_conversation(ctx.cluster, ctx.temp_db.name(), conv_id).await?;

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

    let req_ctx = test_request_context;

    // Store first message
    ctx.repo.store(&req_ctx, &message1).await?;

    // Second store should fail with DuplicateSequence
    let result = ctx.repo.store(&req_ctx, &message2).await;
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
    Ok(())
}

// ============================================================================
// Audit Context Setting Tests
// ============================================================================

/// Tests that `set_audit_context` correctly sets `PostgreSQL` session variables.
///
/// Parameterized across two scenarios:
/// - With causation: all context fields propagated
/// - Without causation: `causation_id` absent from audit log
#[rstest]
#[case::with_causation(true, "with_causation")]
#[case::without_causation(false, "without_causation")]
#[tokio::test]
async fn set_audit_context_propagates_fields(
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
    clock: DefaultClock,
    test_request_context: RequestContext,
    #[case] include_causation: bool,
    #[case] scenario: &str,
) -> Result<(), BoxError> {
    let ctx = prepared_repo.await?;

    let conv_id = ConversationId::new();
    insert_conversation(ctx.cluster, ctx.temp_db.name(), conv_id).await?;

    let mut req_ctx = test_request_context;
    if include_causation {
        req_ctx = req_ctx.with_causation_id(CausationId::new());
    }

    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audit test message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    ctx.repo.store_with_audit(&req_ctx, &message).await?;

    // Verify via audit log
    let audit_log =
        fetch_audit_log_for_message(ctx.cluster, ctx.temp_db.name(), message.id().into_inner())
            .await?
            .expect("audit log should exist");

    assert_eq!(
        audit_log.correlation_id,
        Some(req_ctx.correlation_id().into_inner()),
        "scenario: {scenario}"
    );
    assert_eq!(
        audit_log.causation_id,
        req_ctx.causation_id().map(CausationId::into_inner),
        "scenario: {scenario}"
    );
    assert_eq!(
        audit_log.user_id,
        Some(req_ctx.user_id().into_inner()),
        "scenario: {scenario}"
    );
    assert_eq!(
        audit_log.session_id,
        Some(req_ctx.session_id().into_inner()),
        "scenario: {scenario}"
    );
    Ok(())
}

// ============================================================================
// Insert Message Tests
// ============================================================================

/// Tests that `insert_message` successfully inserts a valid message.
#[rstest]
#[tokio::test]
async fn insert_message_succeeds_for_valid_message(
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
    clock: DefaultClock,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let ctx = prepared_repo.await?;

    let conv_id = ConversationId::new();
    insert_conversation(ctx.cluster, ctx.temp_db.name(), conv_id).await?;

    let message = Message::new(
        conv_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Hello from assistant"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let req_ctx = test_request_context;
    ctx.repo.store(&req_ctx, &message).await?;

    // Verify the message was stored
    let retrieved = ctx
        .repo
        .find_by_id(&req_ctx, message.id())
        .await?
        .expect("message should exist");

    assert_eq!(retrieved.id(), message.id());
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::Assistant);
    Ok(())
}

/// Tests that generic database errors (not constraint violations) are wrapped correctly.
#[rstest]
#[tokio::test]
async fn insert_message_wraps_generic_database_errors(
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
    clock: DefaultClock,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let ctx = prepared_repo.await?;

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

    let req_ctx = test_request_context;
    let result = ctx.repo.store(&req_ctx, &message).await;

    // Should get a Database error (not DuplicateMessage or DuplicateSequence)
    match result {
        Err(RepositoryError::Database(_)) => {
            // Expected - foreign key violation is wrapped as Database error
        }
        other => panic!("expected Database error for FK violation, got {other:?}"),
    }
    Ok(())
}
