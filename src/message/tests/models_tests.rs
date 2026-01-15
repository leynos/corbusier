//! Tests for adapter model types, specifically `NewMessage::try_from_domain`.

use crate::message::{
    adapters::models::NewMessage,
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    error::RepositoryError,
};
use mockable::DefaultClock;
use rstest::rstest;

/// Creates a valid test message with the given sequence number.
fn create_test_message(sequence: u64) -> Message {
    let clock = DefaultClock;
    Message::new(
        ConversationId::new(),
        Role::User,
        vec![ContentPart::Text(TextPart::new("Test content"))],
        SequenceNumber::new(sequence),
        &clock,
    )
    .expect("valid message")
}

#[rstest]
fn try_from_domain_succeeds_for_valid_message() {
    let message = create_test_message(1);

    let result = NewMessage::try_from_domain(&message);

    assert!(result.is_ok());
    let new_message = result.expect("conversion should succeed");
    assert_eq!(new_message.id, message.id().into_inner());
    assert_eq!(
        new_message.conversation_id,
        message.conversation_id().into_inner()
    );
    assert_eq!(new_message.role, "user");
    assert_eq!(new_message.sequence_number, 1);
}

#[rstest]
fn try_from_domain_preserves_all_fields() {
    let clock = DefaultClock;
    let message = Message::new(
        ConversationId::new(),
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Response"))],
        SequenceNumber::new(42),
        &clock,
    )
    .expect("valid message");

    let new_message = NewMessage::try_from_domain(&message).expect("conversion should succeed");

    assert_eq!(new_message.id, message.id().into_inner());
    assert_eq!(
        new_message.conversation_id,
        message.conversation_id().into_inner()
    );
    assert_eq!(new_message.role, "assistant");
    assert_eq!(new_message.sequence_number, 42);
    assert_eq!(new_message.created_at, message.created_at());

    // Verify content is serialised correctly
    let content: Vec<ContentPart> =
        serde_json::from_value(new_message.content).expect("content should deserialise");
    assert_eq!(content.len(), 1);
}

#[rstest]
fn try_from_domain_handles_large_sequence_within_i64() {
    let max_i64_as_u64: u64 = i64::MAX as u64;
    let message = create_test_message(max_i64_as_u64);

    let result = NewMessage::try_from_domain(&message);

    assert!(result.is_ok());
    let new_message = result.expect("conversion should succeed");
    assert_eq!(new_message.sequence_number, i64::MAX);
}

#[rstest]
fn try_from_domain_fails_for_sequence_overflow() {
    // Sequence number larger than i64::MAX
    let overflow_value: u64 = u64::MAX;
    let message = create_test_message(overflow_value);

    let result = NewMessage::try_from_domain(&message);

    assert!(result.is_err());
    let err = result.expect_err("should fail for overflow");
    match err {
        RepositoryError::Serialization(msg) => {
            assert!(
                msg.contains("out of range"),
                "error should mention range: {msg}"
            );
        }
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
#[case(Role::User, "user")]
#[case(Role::Assistant, "assistant")]
#[case(Role::Tool, "tool")]
#[case(Role::System, "system")]
fn try_from_domain_serializes_role_correctly(#[case] role: Role, #[case] expected: &str) {
    let clock = DefaultClock;
    let message = Message::new(
        ConversationId::new(),
        role,
        vec![ContentPart::Text(TextPart::new("Content"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let new_message = NewMessage::try_from_domain(&message).expect("conversion should succeed");

    assert_eq!(new_message.role, expected);
}

#[rstest]
fn try_from_domain_serializes_metadata_correctly() {
    let clock = DefaultClock;
    let message = Message::new(
        ConversationId::new(),
        Role::User,
        vec![ContentPart::Text(TextPart::new("Test"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let new_message = NewMessage::try_from_domain(&message).expect("conversion should succeed");

    // Verify metadata is valid JSON
    assert!(new_message.metadata.is_object());
}
