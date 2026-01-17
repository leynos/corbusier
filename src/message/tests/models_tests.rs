//! Tests for `NewMessage` adapter model type.
//!
//! Covers `NewMessage::try_from_domain` conversion, field preservation, and
//! error handling. Related tests for `ConversationRow`, `NewConversation`,
//! `DomainEventRow`, and `NewDomainEvent` are in their respective modules.
//! Row-to-domain conversion tests are in
//! [`row_to_message_tests`](super::row_to_message_tests).

use crate::message::{
    adapters::models::NewMessage,
    domain::{
        ContentPart, ConversationId, Message, MessageBuilderError, Role, SequenceNumber, TextPart,
    },
    error::RepositoryError,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

/// Provides a [`DefaultClock`] for test fixtures.
#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

/// Factory fixture for creating test messages with configurable sequence numbers.
///
/// Returns a closure that creates valid [`Message`] instances with the specified
/// sequence number, using the injected clock for timestamp generation.
///
/// The closure returns `Result` so test code can use `.expect()` with descriptive
/// messages when the fixture is called.
#[fixture]
fn message_factory(clock: DefaultClock) -> impl Fn(u64) -> Result<Message, MessageBuilderError> {
    move |sequence| {
        Message::new(
            ConversationId::new(),
            Role::User,
            vec![ContentPart::Text(TextPart::new("Test content"))],
            SequenceNumber::new(sequence),
            &clock,
        )
    }
}

#[rstest]
fn try_from_domain_succeeds_for_valid_message(
    message_factory: impl Fn(u64) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(1).expect("fixture should create valid message");

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
fn try_from_domain_handles_large_sequence_within_i64(
    message_factory: impl Fn(u64) -> Result<Message, MessageBuilderError>,
) {
    let max_i64_as_u64: u64 = u64::try_from(i64::MAX).expect("i64::MAX should fit in u64");
    let message = message_factory(max_i64_as_u64).expect("fixture should create valid message");

    let result = NewMessage::try_from_domain(&message);

    assert!(result.is_ok());
    let new_message = result.expect("conversion should succeed");
    assert_eq!(new_message.sequence_number, i64::MAX);
}

#[rstest]
fn try_from_domain_fails_for_sequence_overflow(
    message_factory: impl Fn(u64) -> Result<Message, MessageBuilderError>,
) {
    // Sequence number larger than i64::MAX
    let overflow_value: u64 = u64::MAX;
    let message = message_factory(overflow_value).expect("fixture should create valid message");

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
