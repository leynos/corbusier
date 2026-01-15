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
    message_factory: impl Fn(Role, Vec<ContentPart>) -> Message,
    #[case] role: Role,
    #[case] content: ContentPart,
) {
    let message = message_factory(role, vec![content]);
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
