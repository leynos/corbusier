//! Tests for adapter model types.
//!
//! Covers `NewMessage`, `MessageRow`, `NewConversation`, and `row_to_message`
//! constructors, field preservation, and conversion semantics.

use crate::message::{
    adapters::models::{MessageRow, NewConversation, NewMessage},
    adapters::postgres::row_to_message,
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    error::RepositoryError,
};
use chrono::Utc;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

/// Provides a [`DefaultClock`] for test fixtures.
#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

/// Factory fixture for creating test messages with configurable sequence numbers.
///
/// Returns a closure that creates valid [`Message`] instances with the specified
/// sequence number, using the injected clock for timestamp generation.
#[fixture]
fn message_factory(clock: DefaultClock) -> impl Fn(u64) -> Message {
    move |sequence| {
        Message::new(
            ConversationId::new(),
            Role::User,
            vec![ContentPart::Text(TextPart::new("Test content"))],
            SequenceNumber::new(sequence),
            &clock,
        )
        .expect("valid message")
    }
}

#[rstest]
fn try_from_domain_succeeds_for_valid_message(message_factory: impl Fn(u64) -> Message) {
    let message = message_factory(1);

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
fn try_from_domain_preserves_all_fields(clock: DefaultClock) {
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

    // Verify content is serialized correctly
    let content: Vec<ContentPart> =
        serde_json::from_value(new_message.content).expect("content should deserialize");
    assert_eq!(content.len(), 1);
}

#[rstest]
fn try_from_domain_handles_large_sequence_within_i64(message_factory: impl Fn(u64) -> Message) {
    let max_i64_as_u64: u64 = u64::try_from(i64::MAX).expect("i64::MAX should fit in u64");
    let message = message_factory(max_i64_as_u64);

    let result = NewMessage::try_from_domain(&message);

    assert!(result.is_ok());
    let new_message = result.expect("conversion should succeed");
    assert_eq!(new_message.sequence_number, i64::MAX);
}

#[rstest]
fn try_from_domain_fails_for_sequence_overflow(message_factory: impl Fn(u64) -> Message) {
    // Sequence number larger than i64::MAX
    let overflow_value: u64 = u64::MAX;
    let message = message_factory(overflow_value);

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

/// Placeholder test documenting JSON serialization failure coverage.
///
/// # Why This Test Is Ignored
///
/// The `NewMessage::try_from_domain` function has three potential serialization
/// failure points:
///
/// 1. **Content serialization** (`serde_json::to_value(message.content())`)
/// 2. **Metadata serialization** (`serde_json::to_value(message.metadata())`)
/// 3. **Sequence number conversion** (`i64::try_from(...)`)
///
/// Path (3) is exercised by `try_from_domain_fails_for_sequence_overflow` above,
/// which validates that `RepositoryError::Serialization` is correctly constructed
/// and returned for conversion failures.
///
/// Paths (1) and (2) cannot fail under normal conditions because:
///
/// - `ContentPart` variants (`TextPart`, `ToolCallPart`, `ToolResultPart`,
///   `AttachmentPart`) use `#[derive(Serialize)]` with stable field types
///   (`String`, `serde_json::Value`, `bool`, `Option<T>`)
/// - `MessageMetadata` similarly uses only stable serializable types
/// - `serde_json::to_value` for these types only fails on recursion limits
///   (default 128 levels) or I/O errors (not applicable for in-memory
///   serialization)
///
/// To test these paths would require either:
///
/// - Injecting a mock `Serialize` implementation via `cfg(test)` conditionals
///   in the domain layer (breaks domain/adapter separation)
/// - Using `unsafe` to corrupt memory layout (inappropriate for unit tests)
/// - Constructing pathologically deep structures exceeding recursion limits
///   (not representative of real-world failures)
///
/// # Decision
///
/// JSON serialization failure for content/metadata is **deferred** because:
///
/// 1. The `RepositoryError::Serialization` variant construction is already
///    exercised by the sequence overflow test
/// 2. A serialization failure in `serde_json::to_value` for these types would
///    indicate a critical serde bug, not a domain logic failure
/// 3. The domain types are designed to be always-serializable by construction
///
/// If the domain API evolves to support custom content types with fallible
/// serialization, this test should be implemented.
#[test]
#[ignore = "JSON serialization failure for content/metadata requires domain API changes for failure injection"]
fn try_from_domain_json_serialization_failure_placeholder() {
    // This test documents that JSON serialization failure paths exist but cannot
    // be exercised without domain layer changes. See docstring for rationale.
    //
    // The sequence overflow test above validates RepositoryError::Serialization
    // construction and error handling for the same code path pattern.
}

#[rstest]
#[case(Role::User, "user")]
#[case(Role::Assistant, "assistant")]
#[case(Role::Tool, "tool")]
#[case(Role::System, "system")]
fn try_from_domain_serializes_role_correctly(
    clock: DefaultClock,
    #[case] role: Role,
    #[case] expected: &str,
) {
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
fn try_from_domain_serializes_metadata_correctly(clock: DefaultClock) {
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

// ============================================================================
// NewConversation Tests
// ============================================================================

#[rstest]
fn new_conversation_sets_default_state() {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let conv = NewConversation::new(id, now);

    assert_eq!(conv.id, id);
    assert_eq!(conv.state, "active");
    assert!(conv.task_id.is_none());
    assert_eq!(conv.created_at, now);
    assert_eq!(conv.updated_at, now);
}

#[rstest]
fn new_conversation_has_empty_context() {
    let id = Uuid::new_v4();
    let now = Utc::now();

    let conv = NewConversation::new(id, now);

    assert!(conv.context.is_object());
    let context_obj = conv.context.as_object().expect("context should be object");
    assert!(context_obj.is_empty());
}

// ============================================================================
// MessageRow to Domain Conversion Tests (row_to_message)
// ============================================================================

/// Provides a valid [`MessageRow`] for testing row-to-domain conversions.
///
/// Tests can override individual fields using struct update syntax:
/// `MessageRow { role: "assistant".to_owned(), ..message_row() }`.
#[fixture]
fn message_row() -> MessageRow {
    MessageRow {
        id: Uuid::new_v4(),
        conversation_id: Uuid::new_v4(),
        role: "user".to_owned(),
        content: json!([{"type": "text", "text": "Hello world"}]),
        metadata: json!({}),
        created_at: Utc::now(),
        sequence_number: 1,
    }
}

#[rstest]
fn row_to_message_converts_valid_row(message_row: MessageRow) {
    let expected_id = message_row.id;
    let expected_conv_id = message_row.conversation_id;

    let result = row_to_message(message_row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(message.id().into_inner(), expected_id);
    assert_eq!(message.conversation_id().into_inner(), expected_conv_id);
    assert_eq!(message.role(), Role::User);
    assert_eq!(message.sequence_number().value(), 1);
}

#[rstest]
#[case("user", Role::User)]
#[case("assistant", Role::Assistant)]
#[case("tool", Role::Tool)]
#[case("system", Role::System)]
fn row_to_message_parses_all_role_variants(
    message_row: MessageRow,
    #[case] role_str: &str,
    #[case] expected_role: Role,
) {
    let row = MessageRow {
        role: role_str.to_owned(),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    assert_eq!(
        result.expect("conversion should succeed").role(),
        expected_role
    );
}

#[rstest]
fn row_to_message_fails_for_invalid_role(message_row: MessageRow) {
    let row = MessageRow {
        role: "invalid_role".to_owned(),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_err());
    match result.expect_err("should fail for invalid role") {
        RepositoryError::Serialization(msg) => {
            assert!(
                msg.contains("invalid_role"),
                "error should mention role: {msg}"
            );
        }
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
fn row_to_message_fails_for_empty_content(message_row: MessageRow) {
    let row = MessageRow {
        content: json!([]),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_err());
    match result.expect_err("should fail for empty content") {
        RepositoryError::Serialization(msg) => {
            assert!(
                msg.contains("empty") || msg.contains("content"),
                "error should mention empty content: {msg}"
            );
        }
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
fn row_to_message_fails_for_malformed_content_json(message_row: MessageRow) {
    let row = MessageRow {
        content: json!("not an array"),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_err());
    match result.expect_err("should fail for malformed JSON") {
        RepositoryError::Serialization(_) => {}
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
fn row_to_message_fails_for_negative_sequence_number(message_row: MessageRow) {
    let row = MessageRow {
        sequence_number: -1,
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_err());
    match result.expect_err("should fail for negative sequence") {
        RepositoryError::Serialization(msg) => {
            assert!(
                msg.contains("out of range") || msg.contains("negative"),
                "error should mention range: {msg}"
            );
        }
        other => panic!("expected Serialization error, got {other:?}"),
    }
}

#[rstest]
fn row_to_message_handles_max_valid_sequence_number(message_row: MessageRow) {
    let row = MessageRow {
        sequence_number: i64::MAX,
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    let expected_value = u64::try_from(i64::MAX).expect("i64::MAX should fit in u64");
    assert_eq!(message.sequence_number().value(), expected_value);
}

#[rstest]
fn row_to_message_preserves_timestamp(message_row: MessageRow) {
    let timestamp = Utc::now();
    let row = MessageRow {
        created_at: timestamp,
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(message.created_at(), timestamp);
}

#[rstest]
fn row_to_message_deserializes_complex_content(message_row: MessageRow) {
    let row = MessageRow {
        content: json!([
            {"type": "text", "text": "Hello"},
            {"type": "tool_call", "call_id": "call_123", "name": "search", "arguments": {"q": "test"}}
        ]),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(message.content().len(), 2);
}

#[rstest]
fn row_to_message_deserializes_metadata_with_agent_backend(message_row: MessageRow) {
    let row = MessageRow {
        metadata: json!({"agent_backend": "claude-3-opus"}),
        ..message_row
    };

    let result = row_to_message(row);

    assert!(result.is_ok());
    let message = result.expect("conversion should succeed");
    assert_eq!(
        message.metadata().agent_backend,
        Some("claude-3-opus".to_owned())
    );
}
