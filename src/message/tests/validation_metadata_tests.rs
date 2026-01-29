//! Unit tests for metadata validation rules.

use super::validation_fixtures::{clock, default_validator};
use crate::message::{
    domain::{
        AgentResponseAudit, AgentResponseStatus, ContentPart, ConversationId, Message,
        MessageMetadata, Role, SequenceNumber, TextPart, ToolCallAudit, ToolCallStatus,
    },
    error::ValidationError,
    ports::validator::MessageValidator,
};
use mockable::DefaultClock;
use rstest::rstest;

fn build_message_with_metadata(clock: &DefaultClock, metadata: MessageMetadata) -> Message {
    Message::builder(
        ConversationId::new(),
        Role::Assistant,
        SequenceNumber::new(1),
    )
    .with_content(ContentPart::Text(TextPart::new("Audit test")))
    .with_metadata(metadata)
    .build(clock)
    .expect("test message should build")
}

fn assert_invalid_metadata(result: Result<(), ValidationError>, expected_fragment: &str) {
    match result {
        Err(ValidationError::InvalidMetadata(message)) => {
            assert!(
                message.contains(expected_fragment),
                "expected fragment '{expected_fragment}' in '{message}'"
            );
        }
        Err(ValidationError::Multiple(errors)) => {
            let combined = errors
                .into_iter()
                .filter_map(|error| match error {
                    ValidationError::InvalidMetadata(message) => Some(message),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(";");
            assert!(
                combined.contains(expected_fragment),
                "expected fragment '{expected_fragment}' in '{combined}'"
            );
        }
        Ok(()) => panic!("expected metadata validation to fail"),
        Err(other) => panic!("expected InvalidMetadata, got {other:?}"),
    }
}

#[rstest]
fn validate_metadata_accepts_audit_records(
    clock: DefaultClock,
    default_validator: crate::message::validation::service::DefaultMessageValidator,
) {
    let metadata = MessageMetadata::empty()
        .with_tool_call_audit(ToolCallAudit::new(
            "call-1",
            "read_file",
            ToolCallStatus::Succeeded,
        ))
        .with_agent_response_audit(
            AgentResponseAudit::new(AgentResponseStatus::Completed).with_response_id("resp-1"),
        );

    let message = build_message_with_metadata(&clock, metadata);

    assert!(default_validator.validate_structure(&message).is_ok());
}

#[rstest]
fn validate_metadata_rejects_empty_tool_call_id(
    clock: DefaultClock,
    default_validator: crate::message::validation::service::DefaultMessageValidator,
) {
    let metadata = MessageMetadata::empty().with_tool_call_audit(ToolCallAudit::new(
        "",
        "read_file",
        ToolCallStatus::Queued,
    ));

    let message = build_message_with_metadata(&clock, metadata);
    let result = default_validator.validate_structure(&message);

    assert_invalid_metadata(result, "call_id");
}

#[rstest]
fn validate_metadata_rejects_empty_tool_name(
    clock: DefaultClock,
    default_validator: crate::message::validation::service::DefaultMessageValidator,
) {
    let metadata = MessageMetadata::empty().with_tool_call_audit(ToolCallAudit::new(
        "call-1",
        "",
        ToolCallStatus::Queued,
    ));

    let message = build_message_with_metadata(&clock, metadata);
    let result = default_validator.validate_structure(&message);

    assert_invalid_metadata(result, "tool_name");
}

#[rstest]
fn validate_metadata_rejects_empty_response_id(
    clock: DefaultClock,
    default_validator: crate::message::validation::service::DefaultMessageValidator,
) {
    let metadata = MessageMetadata::empty().with_agent_response_audit(
        AgentResponseAudit::new(AgentResponseStatus::Completed).with_response_id(""),
    );

    let message = build_message_with_metadata(&clock, metadata);
    let result = default_validator.validate_structure(&message);

    assert_invalid_metadata(result, "response_id");
}
