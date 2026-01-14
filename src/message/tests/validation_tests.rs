//! Unit tests for validation service.

use crate::message::{
    domain::{
        AttachmentPart, ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart,
        ToolCallPart, ToolResultPart,
    },
    error::ValidationError,
    ports::validator::{MessageValidator, ValidationConfig},
    validation::service::DefaultMessageValidator,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
fn default_validator() -> DefaultMessageValidator {
    DefaultMessageValidator::new()
}

#[fixture]
fn lenient_validator() -> DefaultMessageValidator {
    DefaultMessageValidator::with_config(ValidationConfig::lenient())
}

#[fixture]
fn strict_validator() -> DefaultMessageValidator {
    DefaultMessageValidator::with_config(ValidationConfig::strict())
}

fn create_message(role: Role, content: Vec<ContentPart>) -> Message {
    let clock = DefaultClock;
    Message::new(
        ConversationId::new(),
        role,
        content,
        SequenceNumber::new(1),
        &clock,
    )
    .expect("test message should build")
}

// ============================================================================
// Structure validation tests
// ============================================================================

#[rstest]
fn valid_text_message_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(Role::User, vec![ContentPart::Text(TextPart::new("Hello"))]);
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_assistant_message_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Here is my response"))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_tool_message_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-123",
            json!({"result": "success"}),
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_system_message_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::System,
        vec![ContentPart::Text(TextPart::new(
            "You are a helpful assistant",
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

// ============================================================================
// Content validation tests - Text
// ============================================================================

#[rstest]
fn empty_text_fails_with_default_config(default_validator: DefaultMessageValidator) {
    let message = create_message(Role::User, vec![ContentPart::Text(TextPart::new(""))]);
    let result = default_validator.validate(&message);
    assert!(result.is_err());
}

#[rstest]
fn whitespace_only_text_fails_with_default_config(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Text(TextPart::new("   \n\t"))],
    );
    let result = default_validator.validate(&message);
    assert!(result.is_err());
}

#[rstest]
fn empty_text_passes_with_lenient_config(lenient_validator: DefaultMessageValidator) {
    let message = create_message(Role::User, vec![ContentPart::Text(TextPart::new(""))]);
    assert!(lenient_validator.validate(&message).is_ok());
}

#[rstest]
fn text_exceeding_max_length_fails(strict_validator: DefaultMessageValidator) {
    // Strict config has max_text_length of 10_000
    let long_text = "x".repeat(10_001);
    let message = create_message(
        Role::User,
        vec![ContentPart::Text(TextPart::new(long_text))],
    );
    let result = strict_validator.validate(&message);
    assert!(result.is_err());
}

// ============================================================================
// Content validation tests - Tool calls
// ============================================================================

#[rstest]
fn valid_tool_call_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(
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
fn tool_call_without_call_id_fails(default_validator: DefaultMessageValidator) {
    let message = create_message(
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
fn tool_call_without_name_fails(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::Assistant,
        vec![ContentPart::ToolCall(ToolCallPart::new(
            "call-123",
            "",
            json!({}),
        ))],
    );
    let result = default_validator.validate(&message);
    assert!(result.is_err());
}

// ============================================================================
// Content validation tests - Tool results
// ============================================================================

#[rstest]
fn valid_tool_result_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-123",
            json!({"output": "file contents"}),
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn tool_result_failure_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::failure(
            "call-123",
            "File not found",
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn tool_result_without_call_id_fails(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "",
            json!({}),
        ))],
    );
    let result = default_validator.validate(&message);
    assert!(result.is_err());
}

// ============================================================================
// Content validation tests - Attachments
// ============================================================================

#[rstest]
fn valid_attachment_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new(
            "text/plain",
            "SGVsbG8gV29ybGQ=",
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn attachment_without_mime_type_fails(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new("", "data"))],
    );
    let result = default_validator.validate(&message);
    assert!(result.is_err());
}

#[rstest]
fn attachment_without_data_fails(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Attachment(AttachmentPart::new(
            "text/plain",
            "",
        ))],
    );
    let result = default_validator.validate(&message);
    assert!(result.is_err());
}

// ============================================================================
// Multiple content parts tests
// ============================================================================

#[rstest]
fn multiple_valid_parts_pass(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("Here are the results:")),
            ContentPart::ToolCall(ToolCallPart::new("call-1", "tool_a", json!({}))),
            ContentPart::ToolCall(ToolCallPart::new("call-2", "tool_b", json!({}))),
        ],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn multiple_errors_collected(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("")), // Invalid: empty text
            ContentPart::ToolCall(ToolCallPart::new("", "tool", json!({}))), // Invalid: no call_id
        ],
    );
    let result = default_validator.validate(&message);

    // Should collect exactly 2 errors
    match result {
        Err(ValidationError::Multiple(errors)) => {
            assert_eq!(errors.len(), 2, "Expected exactly 2 validation errors");
        }
        Err(other) => panic!("Expected Multiple error, got: {other:?}"),
        Ok(()) => panic!("Expected validation to fail"),
    }
}

// ============================================================================
// Content parts limit tests
// ============================================================================

#[rstest]
fn message_exceeding_max_content_parts_fails() {
    // Strict config has max_content_parts of 20
    let config = ValidationConfig::strict();
    let validator = DefaultMessageValidator::with_config(config);

    // Create 21 content parts (exceeds limit of 20)
    let parts: Vec<ContentPart> = (0..21)
        .map(|i| ContentPart::Text(TextPart::new(format!("Part {i}"))))
        .collect();

    let message = create_message(Role::User, parts);
    let result = validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::TooManyContentParts {
            max: 20,
            actual: 21
        })
    ));
}

// ============================================================================
// Size limit tests
// ============================================================================

#[rstest]
fn message_within_size_limit_passes(default_validator: DefaultMessageValidator) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello, world!"))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn message_exceeding_size_limit_fails() {
    // Create a config with a very small size limit
    let config = ValidationConfig {
        max_message_size_bytes: 100,
        ..Default::default()
    };
    let validator = DefaultMessageValidator::with_config(config);

    // Create a message that exceeds 100 bytes when serialised
    let large_text = "x".repeat(200);
    let message = create_message(
        Role::User,
        vec![ContentPart::Text(TextPart::new(large_text))],
    );

    let result = validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::MessageTooLarge { .. })
    ));
}

// ============================================================================
// Validation layer tests
// ============================================================================

#[rstest]
fn validate_structure_checks_id_and_content(default_validator: DefaultMessageValidator) {
    let message = create_message(Role::User, vec![ContentPart::Text(TextPart::new("test"))]);
    assert!(default_validator.validate_structure(&message).is_ok());
}

#[rstest]
fn validate_content_checks_parts(default_validator: DefaultMessageValidator) {
    let message = create_message(Role::User, vec![ContentPart::Text(TextPart::new("test"))]);
    assert!(default_validator.validate_content(&message).is_ok());
}

// ============================================================================
// Configuration tests
// ============================================================================

#[rstest]
fn default_config_values() {
    let config = ValidationConfig::default();
    assert_eq!(config.max_message_size_bytes, 1024 * 1024);
    assert_eq!(config.max_content_parts, 100);
    assert_eq!(config.max_text_length, 100_000);
    assert!(!config.allow_empty_text);
}

#[rstest]
fn lenient_config_allows_empty_text() {
    let config = ValidationConfig::lenient();
    assert!(config.allow_empty_text);
}

#[rstest]
fn strict_config_has_reduced_limits() {
    let config = ValidationConfig::strict();
    assert_eq!(config.max_message_size_bytes, 256 * 1024);
    assert_eq!(config.max_content_parts, 20);
    assert_eq!(config.max_text_length, 10_000);
}
