//! Unit tests for validation service - limits and multi-part tests.

use super::validation_fixtures::{default_validator, message_factory, strict_validator};
use crate::message::{
    domain::{ContentPart, Message, MessageBuilderError, Role, TextPart, ToolCallPart},
    error::ValidationError,
    ports::validator::{MessageValidator, ValidationConfig},
    validation::service::DefaultMessageValidator,
};
use rstest::rstest;
use serde_json::json;

// ============================================================================
// Multiple content parts tests
// ============================================================================

#[rstest]
fn multiple_valid_parts_pass(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("Here are the results:")),
            ContentPart::ToolCall(ToolCallPart::new("call-1", "tool_a", json!({}))),
            ContentPart::ToolCall(ToolCallPart::new("call-2", "tool_b", json!({}))),
        ],
    )
    .expect("test message should build");
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn multiple_errors_collected(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("")), // Invalid: empty text
            ContentPart::ToolCall(ToolCallPart::new("", "tool", json!({}))), // Invalid: no call_id
        ],
    )
    .expect("test message should build");
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
fn message_exceeding_max_content_parts_fails(
    strict_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    // Strict config has max_content_parts of 20
    // Create 21 content parts (exceeds limit of 20)
    let parts: Vec<ContentPart> = (0..21)
        .map(|i| ContentPart::Text(TextPart::new(format!("Part {i}"))))
        .collect();

    let message = message_factory(Role::User, parts).expect("test message should build");
    let result = strict_validator.validate(&message);
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
fn message_within_size_limit_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    let message = message_factory(
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello, world!"))],
    )
    .expect("test message should build");
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn message_exceeding_size_limit_fails(
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Result<Message, MessageBuilderError>,
) {
    // Create a config with a very small size limit
    let config = ValidationConfig {
        max_message_size_bytes: 100,
        ..Default::default()
    };
    let validator = DefaultMessageValidator::with_config(config);

    // Create a message that exceeds 100 bytes when serialized
    let large_text = "x".repeat(200);
    let message = message_factory(
        Role::User,
        vec![ContentPart::Text(TextPart::new(large_text))],
    )
    .expect("test message should build");

    let result = validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::MessageTooLarge { .. })
    ));
}
