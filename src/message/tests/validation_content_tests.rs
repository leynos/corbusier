//! Unit tests for validation service - content validation tests.

use super::validation_fixtures::{
    default_validator, lenient_validator, message_factory, strict_validator,
};
use crate::message::{
    domain::{
        AttachmentPart, ContentPart, Message, MessageBuilderError, Role, TextPart, ToolCallPart,
        ToolResultPart,
    },
    error::ValidationError,
    ports::validator::MessageValidator,
    validation::service::DefaultMessageValidator,
};
use rstest::rstest;
use serde_json::json;

// ============================================================================
// Text validation tests
// ============================================================================

#[rstest]
fn empty_text_fails_with_default_config(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new(""))])
        .expect("test message should build");
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

#[rstest]
fn whitespace_only_text_fails_with_default_config(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Text(TextPart::new("   \n\t"))],
    )
    .expect("test message should build");
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

#[rstest]
fn empty_text_passes_with_lenient_config(
    lenient_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new(""))])
        .expect("test message should build");
    assert!(lenient_validator.validate(&message).is_ok());
}

#[rstest]
fn text_exceeding_max_length_fails(
    strict_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    // Strict config has max_text_length of 10_000
    let long_text = "x".repeat(10_001);
    let message = message_factory(
        Role::User,
        vec![ContentPart::Text(TextPart::new(long_text))],
    )
    .expect("test message should build");
    let result = strict_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

// ============================================================================
// Tool call validation tests
// ============================================================================

#[rstest]
fn valid_tool_call_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "call-123",
            "read_file",
            json!({"path": "/tmp/test.txt"}),
        ))],
    )
    .expect("test message should build");
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn tool_call_without_call_id_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "",
            "read_file",
            json!({}),
        ))],
    )
    .expect("test message should build");
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

#[rstest]
fn tool_call_without_name_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "call-123",
            "",
            json!({}),
        ))],
    )
    .expect("test message should build");
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

// ============================================================================
// Tool result validation tests
// ============================================================================

#[rstest]
fn valid_tool_result_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-123",
            json!({"output": "file contents"}),
        ))],
    )
    .expect("test message should build");
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn tool_result_failure_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::failure(
            "call-123",
            "File not found",
        ))],
    )
    .expect("test message should build");
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn tool_result_without_call_id_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "",
            json!({}),
        ))],
    )
    .expect("test message should build");
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

// ============================================================================
// Attachment validation tests
// ============================================================================

#[rstest]
fn valid_attachment_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new(
            "text/plain",
            "SGVsbG8gV29ybGQ=",
        ))],
    )
    .expect("test message should build");
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn attachment_without_mime_type_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new("", "data"))],
    )
    .expect("test message should build");
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

#[rstest]
fn attachment_without_data_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new(
            "text/plain",
            "",
        ))],
    )
    .expect("test message should build");
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}
