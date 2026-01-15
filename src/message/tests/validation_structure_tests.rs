//! Unit tests for validation service - structure validation tests.

use super::validation_fixtures::{default_validator, message_factory};
use crate::message::{
    domain::{ContentPart, Message, Role, TextPart, ToolResultPart},
    ports::validator::MessageValidator,
    validation::service::DefaultMessageValidator,
};
use rstest::rstest;
use serde_json::json;

#[rstest]
fn valid_text_message_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new("Hello"))]);
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_assistant_message_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Here is my response"))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_tool_message_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-123",
            json!({"result": "success"}),
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn valid_system_message_passes(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(
        Role::System,
        vec![ContentPart::Text(TextPart::new(
            "You are a helpful assistant",
        ))],
    );
    assert!(default_validator.validate(&message).is_ok());
}

#[rstest]
fn validate_structure_checks_id_and_content(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new("test"))]);
    assert!(default_validator.validate_structure(&message).is_ok());
}

#[rstest]
fn validate_content_checks_parts(
    default_validator: DefaultMessageValidator,
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
) {
    let message = message_factory(Role::User, vec![ContentPart::Text(TextPart::new("test"))]);
    assert!(default_validator.validate_content(&message).is_ok());
}
