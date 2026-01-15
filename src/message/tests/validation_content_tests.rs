//! Unit tests for validation service - content validation tests.

use super::validation_fixtures::{
    default_validator, lenient_validator, message_factory, strict_validator,
};
use crate::message::{
    domain::{AttachmentPart, ContentPart, Message, Role, TextPart, ToolCallPart, ToolResultPart},
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
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new(""))]);
    let result = default_validator.validate(&message);
    assert!(result.is_err());
}

#[rstest]
fn whitespace_only_text_fails_with_default_config(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Text(TextPart::new("   \n\t"))],
    );
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

#[rstest]
fn empty_text_passes_with_lenient_config(
    lenient_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new(""))]);
    assert!(lenient_validator.validate(&message).is_ok());
}

#[rstest]
fn text_exceeding_max_length_fails(
    strict_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    // Strict config has max_text_length of 10_000
    let long_text = "x".repeat(10_001);
    let message = message_factory(
        Role::User,
        vec![ContentPart::Text(TextPart::new(long_text))],
    );
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
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "call-123",
            "read_file",
            json!({"path": "/tmp/test.txt"}),
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn tool_call_without_call_id_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "",
            "read_file",
            json!({}),
        ))],
    );
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

#[rstest]
fn tool_call_without_name_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "call-123",
            "",
            json!({}),
        ))],
    );
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
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-123",
            json!({"output": "file contents"}),
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn tool_result_failure_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::failure(
            "call-123",
            "File not found",
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn tool_result_without_call_id_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "",
            json!({}),
        ))],
    );
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
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new(
            "text/plain",
            "SGVsbG8gV29ybGQ=",
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn attachment_without_mime_type_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new("", "data"))],
    );
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}

#[rstest]
fn attachment_without_data_fails(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new(
            "text/plain",
            "",
        ))],
    );
    let result = default_validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::InvalidContentPart { index: 0, .. })
    ));
}
