//! Unit tests for validation service - structure validation tests.

use super::validation_fixtures::{clock, create_message, default_validator};
use crate::message::{
    domain::{ContentPart, Role, TextPart, ToolResultPart},
    ports::validator::MessageValidator,
    validation::service::DefaultMessageValidator,
};
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;

#[rstest]
fn valid_text_message_passes(default_validator: DefaultMessageValidator, clock: DefaultClock) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello"))],
        &clock,
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_assistant_message_passes(default_validator: DefaultMessageValidator, clock: DefaultClock) {
    let message = create_message(
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Here is my response"))],
        &clock,
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_tool_message_passes(default_validator: DefaultMessageValidator, clock: DefaultClock) {
    let message = create_message(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-123",
            json!({"result": "success"}),
        ))],
        &clock,
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_system_message_passes(default_validator: DefaultMessageValidator, clock: DefaultClock) {
    let message = create_message(
        Role::System,
        vec![ContentPart::Text(TextPart::new(
            "You are a helpful assistant",
        ))],
        &clock,
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn validate_structure_checks_id_and_content(
    default_validator: DefaultMessageValidator,
    clock: DefaultClock,
) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Text(TextPart::new("test"))],
        &clock,
    );
    assert!(default_validator.validate_structure(&message).is_ok());
}

#[rstest]
fn validate_content_checks_parts(default_validator: DefaultMessageValidator, clock: DefaultClock) {
    let message = create_message(
        Role::User,
        vec![ContentPart::Text(TextPart::new("test"))],
        &clock,
    );
    assert!(default_validator.validate_content(&message).is_ok());
}
