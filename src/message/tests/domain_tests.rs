//! Unit tests for domain types.

#![expect(
    clippy::too_many_arguments,
    reason = "rstest case expansion creates many parameters from #[case] attributes"
)]

use crate::message::domain::{
    AttachmentPart, ContentPart, ConversationId, Message, MessageBuilderError, MessageId,
    MessageMetadata, Role, SequenceNumber, TextPart, ToolCallPart, ToolResultPart, TurnId,
};
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;

// ============================================================================
// MessageId tests
// ============================================================================

#[rstest]
fn message_id_new_creates_non_nil() {
    let id = MessageId::new();
    assert!(!id.as_ref().is_nil());
}

#[rstest]
fn message_id_default_creates_non_nil() {
    let id = MessageId::default();
    assert!(!id.as_ref().is_nil());
}

#[rstest]
fn message_id_different_ids_not_equal() {
    let id1 = MessageId::new();
    let id2 = MessageId::new();
    assert_ne!(id1, id2);
}

#[rstest]
fn message_id_from_uuid_preserves_value() {
    let uuid = uuid::Uuid::new_v4();
    let id = MessageId::from_uuid(uuid);
    assert_eq!(id.as_ref(), &uuid);
    assert_eq!(id.into_inner(), uuid);
}

#[rstest]
fn message_id_display() {
    let uuid =
        uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid UUID string");
    let id = MessageId::from_uuid(uuid);
    assert_eq!(id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
}

// ============================================================================
// ConversationId tests
// ============================================================================

#[rstest]
fn conversation_id_new_creates_non_nil() {
    let id = ConversationId::new();
    assert!(!id.as_ref().is_nil());
}

// ============================================================================
// TurnId tests
// ============================================================================

#[rstest]
fn turn_id_new_creates_non_nil() {
    let id = TurnId::new();
    assert!(!id.as_ref().is_nil());
}

// ============================================================================
// SequenceNumber tests
// ============================================================================

#[rstest]
fn sequence_number_new_stores_value() {
    let seq = SequenceNumber::new(42);
    assert_eq!(seq.value(), 42);
}

#[rstest]
fn sequence_number_next_increments() {
    let seq = SequenceNumber::new(1);
    assert_eq!(seq.next().value(), 2);
}

#[rstest]
fn sequence_number_from_u64() {
    let seq: SequenceNumber = 100.into();
    assert_eq!(seq.value(), 100);
}

#[rstest]
fn sequence_number_ordering() {
    let seq1 = SequenceNumber::new(1);
    let seq2 = SequenceNumber::new(2);
    assert!(seq1 < seq2);
}

// ============================================================================
// Role tests
// ============================================================================

#[rstest]
#[case(Role::User, false, true, false, false)]
#[case(Role::Assistant, true, false, false, false)]
#[case(Role::Tool, false, false, false, true)]
#[case(Role::System, false, false, true, false)]
fn role_capabilities(
    #[case] role: Role,
    #[case] can_call_tools: bool,
    #[case] is_human: bool,
    #[case] is_system: bool,
    #[case] is_tool: bool,
) {
    assert_eq!(role.can_call_tools(), can_call_tools);
    assert_eq!(role.is_human(), is_human);
    assert_eq!(role.is_system(), is_system);
    assert_eq!(role.is_tool(), is_tool);
}

#[rstest]
#[case(Role::User, "user")]
#[case(Role::Assistant, "assistant")]
#[case(Role::Tool, "tool")]
#[case(Role::System, "system")]
fn role_display(#[case] role: Role, #[case] expected: &str) {
    assert_eq!(role.to_string(), expected);
}

#[rstest]
fn role_serialization_round_trip() {
    let roles = [Role::User, Role::Assistant, Role::Tool, Role::System];
    for role in roles {
        let json = serde_json::to_string(&role).expect("serialize");
        let deserialized: Role = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(role, deserialized);
    }
}

// ============================================================================
// TextPart tests
// ============================================================================

#[rstest]
fn text_part_new() {
    let text = TextPart::new("Hello, world!");
    assert_eq!(text.text, "Hello, world!");
}

#[rstest]
#[case("", true)]
#[case("   ", true)]
#[case("\n\t", true)]
#[case("hello", false)]
#[case(" hello ", false)]
fn text_part_is_empty(#[case] content: &str, #[case] expected: bool) {
    let text = TextPart::new(content);
    assert_eq!(text.is_empty(), expected);
}

#[rstest]
fn text_part_len() {
    let text = TextPart::new("hello");
    assert_eq!(text.len(), 5);
}

// ============================================================================
// ToolCallPart tests
// ============================================================================

#[rstest]
fn tool_call_part_new() {
    let call = ToolCallPart::new("call-123", "my_tool", json!({"arg": "value"}));
    assert_eq!(call.call_id, "call-123");
    assert_eq!(call.name, "my_tool");
    assert_eq!(call.arguments, json!({"arg": "value"}));
}

#[rstest]
#[case("call-123", "tool", true)]
#[case("", "tool", false)]
#[case("call-123", "", false)]
#[case("", "", false)]
fn tool_call_is_valid(#[case] call_id: &str, #[case] name: &str, #[case] expected: bool) {
    let call = ToolCallPart::new(call_id, name, json!({}));
    assert_eq!(call.is_valid(), expected);
}

// ============================================================================
// ToolResultPart tests
// ============================================================================

#[rstest]
fn tool_result_success() {
    let result = ToolResultPart::success("call-123", json!({"data": "result"}));
    assert!(result.success);
    assert_eq!(result.call_id, "call-123");
}

#[rstest]
fn tool_result_failure() {
    let result = ToolResultPart::failure("call-123", "Something went wrong");
    assert!(!result.success);
    assert_eq!(result.content, json!("Something went wrong"));
}

#[rstest]
fn tool_result_is_valid() {
    let valid = ToolResultPart::success("call-123", json!({}));
    let invalid = ToolResultPart::success("", json!({}));
    assert!(valid.is_valid());
    assert!(!invalid.is_valid());
}

// ============================================================================
// AttachmentPart tests
// ============================================================================

#[rstest]
fn attachment_part_new() {
    let attachment = AttachmentPart::new("text/plain", "SGVsbG8=");
    assert_eq!(attachment.mime_type, "text/plain");
    assert_eq!(attachment.data, "SGVsbG8=");
    assert!(attachment.name.is_none());
    assert!(attachment.size_bytes.is_none());
}

#[rstest]
fn attachment_part_with_name_and_size() {
    let attachment = AttachmentPart::new("image/png", "data")
        .with_name("image.png")
        .with_size(1024);
    assert_eq!(attachment.name, Some("image.png".to_owned()));
    assert_eq!(attachment.size_bytes, Some(1024));
}

#[rstest]
#[case("text/plain", "data", true)]
#[case("", "data", false)]
#[case("text/plain", "", false)]
fn attachment_is_valid(#[case] mime_type: &str, #[case] data: &str, #[case] expected: bool) {
    let attachment = AttachmentPart::new(mime_type, data);
    assert_eq!(attachment.is_valid(), expected);
}

// ============================================================================
// MessageMetadata tests
// ============================================================================

#[rstest]
fn message_metadata_empty() {
    let metadata = MessageMetadata::empty();
    assert!(metadata.is_empty());
}

#[rstest]
fn message_metadata_with_agent_backend() {
    let metadata = MessageMetadata::with_agent_backend("claude_code_sdk");
    assert_eq!(metadata.agent_backend, Some("claude_code_sdk".to_owned()));
    assert!(!metadata.is_empty());
}

#[rstest]
fn message_metadata_builder_chain() {
    let turn_id = TurnId::new();
    let metadata = MessageMetadata::with_agent_backend("claude")
        .with_turn_id(turn_id)
        .with_extension("custom", json!({"key": "value"}));

    assert_eq!(metadata.agent_backend, Some("claude".to_owned()));
    assert_eq!(metadata.turn_id, Some(turn_id));
    assert!(metadata.extensions.contains_key("custom"));
}

// ============================================================================
// Message tests
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
