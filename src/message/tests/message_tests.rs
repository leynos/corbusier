//! Unit tests for Message aggregate and builder.

use crate::message::domain::{
    ContentPart, ConversationId, Message, MessageBuilderError, MessageId, MessageMetadata, Role,
    SequenceNumber, TextPart, ToolCallPart,
};
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;

// ============================================================================
// Message constructor tests
// ============================================================================

#[rstest]
fn message_new_with_valid_content() {
    let clock = DefaultClock;
    let result = Message::new(
        ConversationId::new(),
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello"))],
        SequenceNumber::new(1),
        &clock,
    );
    assert!(result.is_ok());
}

#[rstest]
fn message_new_with_empty_content_fails() {
    let clock = DefaultClock;
    let result = Message::new(
        ConversationId::new(),
        Role::User,
        vec![],
        SequenceNumber::new(1),
        &clock,
    );
    assert!(matches!(result, Err(MessageBuilderError::EmptyContent)));
}

#[rstest]
fn message_accessors() {
    let clock = DefaultClock;
    let conversation_id = ConversationId::new();
    let seq = SequenceNumber::new(5);
    let message = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Response"))],
        seq,
        &clock,
    )
    .expect("valid message");

    assert!(!message.id().as_ref().is_nil());
    assert_eq!(message.conversation_id(), conversation_id);
    assert_eq!(message.role(), Role::Assistant);
    assert_eq!(message.content().len(), 1);
    assert_eq!(message.sequence_number(), seq);
    // created_at should be set
    assert!(message.created_at().timestamp() > 0);
}

// ============================================================================
// MessageBuilder tests
// ============================================================================

#[rstest]
fn message_builder_basic() {
    let clock = DefaultClock;
    let message = Message::builder(ConversationId::new(), Role::User, SequenceNumber::new(1))
        .with_content(ContentPart::Text(TextPart::new("Hello")))
        .build(&clock)
        .expect("valid message");

    assert_eq!(message.role(), Role::User);
}

#[rstest]
fn message_builder_with_metadata() {
    let clock = DefaultClock;
    let metadata = MessageMetadata::with_agent_backend("claude");
    let message = Message::builder(
        ConversationId::new(),
        Role::Assistant,
        SequenceNumber::new(2),
    )
    .with_content(ContentPart::Text(TextPart::new("Response")))
    .with_metadata(metadata)
    .build(&clock)
    .expect("valid message");

    assert_eq!(message.metadata().agent_backend, Some("claude".to_owned()));
}

#[rstest]
fn message_builder_with_custom_id() {
    let clock = DefaultClock;
    let custom_id = MessageId::new();
    let message = Message::builder(ConversationId::new(), Role::User, SequenceNumber::new(1))
        .with_id(custom_id)
        .with_content(ContentPart::Text(TextPart::new("Hello")))
        .build(&clock)
        .expect("valid message");

    assert_eq!(message.id(), custom_id);
}

#[rstest]
fn message_builder_with_multiple_content_parts() {
    let clock = DefaultClock;
    let message = Message::builder(
        ConversationId::new(),
        Role::Assistant,
        SequenceNumber::new(1),
    )
    .with_content(ContentPart::Text(TextPart::new("Here's the result:")))
    .with_content(ContentPart::ToolCall(ToolCallPart::new(
        "call-1",
        "read_file",
        json!({"path": "/tmp/test"}),
    )))
    .build(&clock)
    .expect("valid message");

    assert_eq!(message.content().len(), 2);
}

#[rstest]
fn message_builder_with_content_parts_iterator() {
    let clock = DefaultClock;
    let parts = vec![
        ContentPart::Text(TextPart::new("Part 1")),
        ContentPart::Text(TextPart::new("Part 2")),
    ];
    let message = Message::builder(ConversationId::new(), Role::User, SequenceNumber::new(1))
        .with_content_parts(parts)
        .build(&clock)
        .expect("valid message");

    assert_eq!(message.content().len(), 2);
}

#[rstest]
fn message_builder_empty_content_fails() {
    let clock = DefaultClock;
    let result =
        Message::builder(ConversationId::new(), Role::User, SequenceNumber::new(1)).build(&clock);

    assert!(matches!(result, Err(MessageBuilderError::EmptyContent)));
}

// ============================================================================
// Serialization tests
// ============================================================================

#[rstest]
fn message_serialization_round_trip() {
    let clock = DefaultClock;
    let message = Message::new(
        ConversationId::new(),
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let json = serde_json::to_string(&message).expect("serialize");
    let deserialized: Message = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(message.id(), deserialized.id());
    assert_eq!(message.role(), deserialized.role());
}
