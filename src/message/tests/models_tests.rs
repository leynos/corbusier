//! Tests for adapter model types.
//!
//! Covers `NewMessage`, `NewConversation`, `ConversationRow`, `DomainEventRow`,
//! and `NewDomainEvent` struct construction, field preservation, and conversion
//! semantics. Row-to-domain conversion tests are in
//! [`row_to_message_tests`](super::row_to_message_tests).

use crate::message::{
    adapters::models::{
        ConversationRow, DomainEventRow, NewConversation, NewDomainEvent, NewMessage,
    },
    domain::{
        ContentPart, ConversationId, Message, MessageBuilderError, Role, SequenceNumber, TextPart,
    },
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
// ConversationRow Tests
// ============================================================================

#[rstest]
fn conversation_row_struct_holds_all_fields() {
    let id = Uuid::new_v4();
    let task_id = Some(Uuid::new_v4());
    let context = json!({"key": "value"});
    let state = "active".to_owned();
    let created_at = Utc::now();
    let updated_at = Utc::now();

    let row = ConversationRow {
        id,
        task_id,
        context: context.clone(),
        state: state.clone(),
        created_at,
        updated_at,
    };

    assert_eq!(row.id, id);
    assert_eq!(row.task_id, task_id);
    assert_eq!(row.context, context);
    assert_eq!(row.state, state);
    assert_eq!(row.created_at, created_at);
    assert_eq!(row.updated_at, updated_at);
}

#[rstest]
fn conversation_row_clone_preserves_fields() {
    let row = ConversationRow {
        id: Uuid::new_v4(),
        task_id: None,
        context: json!({}),
        state: "completed".to_owned(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let cloned = row.clone();

    assert_eq!(cloned.id, row.id);
    assert_eq!(cloned.task_id, row.task_id);
    assert_eq!(cloned.context, row.context);
    assert_eq!(cloned.state, row.state);
    assert_eq!(cloned.created_at, row.created_at);
    assert_eq!(cloned.updated_at, row.updated_at);
}

#[rstest]
fn conversation_row_debug_format() {
    let row = ConversationRow {
        id: Uuid::nil(),
        task_id: None,
        context: json!({}),
        state: "active".to_owned(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let debug_str = format!("{row:?}");
    assert!(debug_str.contains("ConversationRow"));
    assert!(debug_str.contains("active"));
}

// ============================================================================
// DomainEventRow Tests
// ============================================================================

#[rstest]
fn domain_event_row_struct_holds_all_fields() {
    let id = Uuid::new_v4();
    let aggregate_id = Uuid::new_v4();
    let aggregate_type = "Message".to_owned();
    let event_type = "MessageCreated".to_owned();
    let event_data = json!({"content": "test"});
    let event_version = 2;
    let occurred_at = Utc::now();
    let correlation_id = Some(Uuid::new_v4());
    let causation_id = Some(Uuid::new_v4());
    let user_id = Some(Uuid::new_v4());
    let session_id = Some(Uuid::new_v4());

    let row = DomainEventRow {
        id,
        aggregate_id,
        aggregate_type: aggregate_type.clone(),
        event_type: event_type.clone(),
        event_data: event_data.clone(),
        event_version,
        occurred_at,
        correlation_id,
        causation_id,
        user_id,
        session_id,
    };

    assert_eq!(row.id, id);
    assert_eq!(row.aggregate_id, aggregate_id);
    assert_eq!(row.aggregate_type, aggregate_type);
    assert_eq!(row.event_type, event_type);
    assert_eq!(row.event_data, event_data);
    assert_eq!(row.event_version, event_version);
    assert_eq!(row.occurred_at, occurred_at);
    assert_eq!(row.correlation_id, correlation_id);
    assert_eq!(row.causation_id, causation_id);
    assert_eq!(row.user_id, user_id);
    assert_eq!(row.session_id, session_id);
}

#[rstest]
fn domain_event_row_with_no_audit_fields() {
    let row = DomainEventRow {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: "Conversation".to_owned(),
        event_type: "ConversationStarted".to_owned(),
        event_data: json!({}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    };

    assert!(row.correlation_id.is_none());
    assert!(row.causation_id.is_none());
    assert!(row.user_id.is_none());
    assert!(row.session_id.is_none());
}

#[rstest]
fn domain_event_row_clone_preserves_all_fields() {
    let row = DomainEventRow {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: "Message".to_owned(),
        event_type: "MessageDeleted".to_owned(),
        event_data: json!({"reason": "spam"}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: Some(Uuid::new_v4()),
        causation_id: None,
        user_id: Some(Uuid::new_v4()),
        session_id: None,
    };

    let cloned = row.clone();

    assert_eq!(cloned.id, row.id);
    assert_eq!(cloned.aggregate_id, row.aggregate_id);
    assert_eq!(cloned.aggregate_type, row.aggregate_type);
    assert_eq!(cloned.event_type, row.event_type);
    assert_eq!(cloned.event_data, row.event_data);
    assert_eq!(cloned.event_version, row.event_version);
    assert_eq!(cloned.occurred_at, row.occurred_at);
    assert_eq!(cloned.correlation_id, row.correlation_id);
    assert_eq!(cloned.causation_id, row.causation_id);
    assert_eq!(cloned.user_id, row.user_id);
    assert_eq!(cloned.session_id, row.session_id);
}

#[rstest]
fn domain_event_row_debug_format() {
    let row = DomainEventRow {
        id: Uuid::nil(),
        aggregate_id: Uuid::nil(),
        aggregate_type: "Test".to_owned(),
        event_type: "TestEvent".to_owned(),
        event_data: json!({}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    };

    let debug_str = format!("{row:?}");
    assert!(debug_str.contains("DomainEventRow"));
    assert!(debug_str.contains("TestEvent"));
}

// ============================================================================
// NewDomainEvent Tests
// ============================================================================

#[rstest]
fn new_domain_event_struct_holds_all_fields() {
    let id = Uuid::new_v4();
    let aggregate_id = Uuid::new_v4();
    let aggregate_type = "Message".to_owned();
    let event_type = "MessageCreated".to_owned();
    let event_data = json!({"content": "hello"});
    let event_version = 1;
    let occurred_at = Utc::now();
    let correlation_id = Some(Uuid::new_v4());
    let causation_id = Some(Uuid::new_v4());
    let user_id = Some(Uuid::new_v4());
    let session_id = Some(Uuid::new_v4());

    let event = NewDomainEvent {
        id,
        aggregate_id,
        aggregate_type: aggregate_type.clone(),
        event_type: event_type.clone(),
        event_data: event_data.clone(),
        event_version,
        occurred_at,
        correlation_id,
        causation_id,
        user_id,
        session_id,
    };

    assert_eq!(event.id, id);
    assert_eq!(event.aggregate_id, aggregate_id);
    assert_eq!(event.aggregate_type, aggregate_type);
    assert_eq!(event.event_type, event_type);
    assert_eq!(event.event_data, event_data);
    assert_eq!(event.event_version, event_version);
    assert_eq!(event.occurred_at, occurred_at);
    assert_eq!(event.correlation_id, correlation_id);
    assert_eq!(event.causation_id, causation_id);
    assert_eq!(event.user_id, user_id);
    assert_eq!(event.session_id, session_id);
}

#[rstest]
fn new_domain_event_with_minimal_fields() {
    let event = NewDomainEvent {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: "Conversation".to_owned(),
        event_type: "ConversationEnded".to_owned(),
        event_data: json!({}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    };

    assert!(event.correlation_id.is_none());
    assert!(event.causation_id.is_none());
    assert!(event.user_id.is_none());
    assert!(event.session_id.is_none());
    assert_eq!(event.event_version, 1);
}

#[rstest]
fn new_domain_event_clone_preserves_all_fields() {
    let event = NewDomainEvent {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        aggregate_type: "Message".to_owned(),
        event_type: "MessageUpdated".to_owned(),
        event_data: json!({"old": "content", "new": "updated"}),
        event_version: 2,
        occurred_at: Utc::now(),
        correlation_id: Some(Uuid::new_v4()),
        causation_id: Some(Uuid::new_v4()),
        user_id: None,
        session_id: Some(Uuid::new_v4()),
    };

    let cloned = event.clone();

    assert_eq!(cloned.id, event.id);
    assert_eq!(cloned.aggregate_id, event.aggregate_id);
    assert_eq!(cloned.aggregate_type, event.aggregate_type);
    assert_eq!(cloned.event_type, event.event_type);
    assert_eq!(cloned.event_data, event.event_data);
    assert_eq!(cloned.event_version, event.event_version);
    assert_eq!(cloned.occurred_at, event.occurred_at);
    assert_eq!(cloned.correlation_id, event.correlation_id);
    assert_eq!(cloned.causation_id, event.causation_id);
    assert_eq!(cloned.user_id, event.user_id);
    assert_eq!(cloned.session_id, event.session_id);
}

#[rstest]
fn new_domain_event_debug_format() {
    let event = NewDomainEvent {
        id: Uuid::nil(),
        aggregate_id: Uuid::nil(),
        aggregate_type: "Test".to_owned(),
        event_type: "TestCreated".to_owned(),
        event_data: json!({}),
        event_version: 1,
        occurred_at: Utc::now(),
        correlation_id: None,
        causation_id: None,
        user_id: None,
        session_id: None,
    };

    let debug_str = format!("{event:?}");
    assert!(debug_str.contains("NewDomainEvent"));
    assert!(debug_str.contains("TestCreated"));
}
