//! Unit tests for validation service - limits and multi-part tests.

use super::validation_fixtures::{clock, create_message, default_validator};
use crate::message::{
    domain::{ContentPart, Role, TextPart, ToolCallPart},
    error::ValidationError,
    ports::validator::{MessageValidator, ValidationConfig},
    validation::service::DefaultMessageValidator,
};
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;

// ============================================================================
// Multiple content parts tests
// ============================================================================

#[rstest]
fn multiple_valid_parts_pass(default_validator: DefaultMessageValidator, clock: DefaultClock) {
    let message = create_message(
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("Here are the results:")),
            ContentPart::ToolCall(ToolCallPart::new("call-1", "tool_a", json!({}))),
            ContentPart::ToolCall(ToolCallPart::new("call-2", "tool_b", json!({}))),
        ],
        &clock,
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn multiple_errors_collected(default_validator: DefaultMessageValidator, clock: DefaultClock) {
    let message = create_message(
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("")), // Invalid: empty text
            ContentPart::ToolCall(ToolCallPart::new("", "tool", json!({}))), // Invalid: no call_id
        ],
        &clock,
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
fn message_exceeding_max_content_parts_fails(clock: DefaultClock) {
    // Strict config has max_content_parts of 20
    let config = ValidationConfig::strict();
    let validator = DefaultMessageValidator::with_config(config);

    // Create 21 content parts (exceeds limit of 20)
    let parts: Vec<ContentPart> = (0..21)
        .map(|i| ContentPart::Text(TextPart::new(format!("Part {i}"))))
        .collect();

    let message = create_message(Role::User, parts, &clock);
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
fn message_within_size_limit_passes(
    default_validator: DefaultMessageValidator,
    clock: DefaultClock,
) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello, world!"))],
        &clock,
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn message_exceeding_size_limit_fails(clock: DefaultClock) {
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
        &clock,
    );

    let result = validator.validate(&message);
    assert!(matches!(
        result,
        Err(ValidationError::MessageTooLarge { .. })
    ));
}
