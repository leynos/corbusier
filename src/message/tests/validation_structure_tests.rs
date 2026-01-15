//! Unit tests for validation service - structure validation tests.

use super::validation_fixtures::{default_validator, message_factory};
use crate::message::{
    domain::{
        ContentPart, Message, MessageBuilderError, Role, TextPart, ToolCallPart, ToolResultPart,
    },
    error::ValidationError,
    ports::validator::MessageValidator,
    validation::service::DefaultMessageValidator,
};
use rstest::rstest;
use serde_json::json;

#[rstest]
#[case::user(Role::User, ContentPart::Text(TextPart::new("Hello")))]
#[case::assistant(
    Role::Assistant,
    ContentPart::Text(TextPart::new("Here is my response"))
)]
#[case::tool(Role::Tool, ContentPart::ToolResult(ToolResultPart::success("call-123", json!({"result": "success"}))))]
#[case::system(
    Role::System,
    ContentPart::Text(TextPart::new("You are a helpful assistant"))
)]
fn valid_message_with_role_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
    #[case] role: Role,
    #[case] content: ContentPart,
) {
    let message = message_factory(role, vec![content]).expect("test message should build");
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn validate_structure_checks_id_and_content(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new("test"))])
        .expect("test message should build");
    assert!(default_validator.validate_structure(&message).is_ok());
}

#[rstest]
fn validate_content_checks_parts(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new("test"))])
        .expect("test message should build");
    assert!(default_validator.validate_content(&message).is_ok());
}

// ============================================================================
// Negative tests - structure validation
// ============================================================================

#[rstest]
fn validate_structure_rejects_nil_id(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    // Create a valid message, then deserialize with a nil ID to bypass constructor checks.
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new("test"))])
        .expect("test message should build");
    let mut json_value: serde_json::Value = serde_json::to_value(&message).expect("serialize");
    *json_value
        .get_mut("id")
        .expect("message should have id field") = json!("00000000-0000-0000-0000-000000000000");
    let nil_id_message: Message =
        serde_json::from_value(json_value).expect("deserialize with nil ID");

    assert!(matches!(
        default_validator.validate_structure(&nil_id_message),
        Err(ValidationError::MissingMessageId)
    ));
}

#[rstest]
fn validate_structure_rejects_empty_content(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    // Create a valid message, then deserialize with empty content to bypass constructor checks.
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new("test"))])
        .expect("test message should build");
    let mut json_value: serde_json::Value = serde_json::to_value(&message).expect("serialize");
    *json_value
        .get_mut("content")
        .expect("message should have content field") = json!([]);
    let empty_content_message: Message =
        serde_json::from_value(json_value).expect("deserialize with empty content");

    assert!(matches!(
        default_validator.validate_structure(&empty_content_message),
        Err(ValidationError::EmptyContent)
    ));
}

// ============================================================================
// Negative tests - content validation
// ============================================================================

#[rstest]
fn validate_content_rejects_invalid_tool_call(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    // ToolCallPart with empty call_id should fail validation.
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "",
            "read_file",
            json!({"path": "/tmp"}),
        ))],
    )
    .expect("test message should build");

    assert!(matches!(
        default_validator.validate_content(&message),
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

#[rstest]
fn validate_content_rejects_tool_call_without_name(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    // ToolCallPart with empty name should fail validation.
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "call-123",
            "",
            json!({"path": "/tmp"}),
        ))],
    )
    .expect("test message should build");

    assert!(matches!(
        default_validator.validate_content(&message),
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}
