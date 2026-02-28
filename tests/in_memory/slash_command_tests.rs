//! In-memory integration tests for slash-command orchestration.

use std::sync::Arc;

use crate::in_memory::helpers::{clock, conversation_id, repo, runtime};
use corbusier::message::{
    adapters::memory::{InMemoryMessageRepository, InMemorySlashCommandRegistry},
    domain::{
        ContentPart, ConversationId, Message, MessageMetadata, Role, SequenceNumber, TextPart,
        ToolCallStatus,
    },
    ports::repository::MessageRepository,
    services::SlashCommandService,
};
use mockable::DefaultClock;
use rstest::rstest;
use tokio::runtime::Runtime;

// Helper functions for assertion reuse across slash command tests
/// Helper struct for tool call audit assertions
struct ExpectedAudit<'a> {
    tool_name: &'a str,
    status: ToolCallStatus,
    call_id_prefix: &'a str,
}

impl<'a> ExpectedAudit<'a> {
    const fn new(tool_name: &'a str, status: ToolCallStatus, call_id_prefix: &'a str) -> Self {
        Self {
            tool_name,
            status,
            call_id_prefix,
        }
    }
}

fn assert_expansion_parameters(
    message: &Message,
    expected_command: &str,
    expected_parameters: &[(&str, &str)],
) {
    let expansion = message
        .metadata()
        .slash_command_expansion
        .as_ref()
        .unwrap_or_else(|| panic!("expected slash command expansion metadata"));
    assert_eq!(expansion.command, expected_command);

    for (key, expected_value) in expected_parameters {
        assert_eq!(
            expansion
                .parameters
                .get(*key)
                .and_then(serde_json::Value::as_str),
            Some(*expected_value),
            "expected slash command parameter `{key}` to equal `{expected_value}`"
        );
    }
}

fn assert_tool_call_audit(message: &Message, audit_index: usize, expected: &ExpectedAudit<'_>) {
    let audit = message
        .metadata()
        .tool_call_audits
        .get(audit_index)
        .unwrap_or_else(|| panic!("expected tool call audit at index {audit_index}"));
    assert_eq!(audit.tool_name, expected.tool_name);
    assert_eq!(&audit.status, &expected.status);
    assert!(
        audit.call_id.starts_with(expected.call_id_prefix),
        "expected call_id `{}` to start with `{}`",
        audit.call_id,
        expected.call_id_prefix
    );
}

#[rstest]
fn slash_command_execution_metadata_round_trip_in_memory(
    runtime: std::io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) {
    let rt = runtime.expect("runtime fixture should initialize");
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let execution = service
        .execute("/task action=start issue=42")
        .expect("slash command should execute");
    let (command_expansion, tool_call_audits) = execution.into_expansion_and_audits();
    let metadata = MessageMetadata::empty()
        .with_slash_command_expansion(command_expansion)
        .with_tool_call_audits(tool_call_audits);

    let message = Message::builder(conversation_id, Role::System, SequenceNumber::new(1))
        .with_content(ContentPart::Text(TextPart::new(
            "Slash command planned for execution",
        )))
        .with_metadata(metadata)
        .build(&clock)
        .expect("message build should succeed");

    rt.block_on(repo.store(&message))
        .expect("storing message should succeed");

    let persisted = rt
        .block_on(repo.find_by_conversation(conversation_id))
        .expect("message lookup should succeed")
        .first()
        .cloned()
        .expect("expected persisted message");

    assert_expansion_parameters(&persisted, "/task", &[("action", "start"), ("issue", "42")]);
    assert_eq!(persisted.metadata().tool_call_audits.len(), 1);
    assert_tool_call_audit(
        &persisted,
        0,
        &ExpectedAudit::new("task_service", ToolCallStatus::Queued, "sc-0-"),
    );
}

#[rstest]
fn slash_command_validation_errors_are_reported() {
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let error = service
        .execute("/task issue=42")
        .expect_err("missing required action parameter should fail");

    assert!(matches!(
        error,
        corbusier::message::domain::SlashCommandError::MissingRequiredParameter {
            command,
            parameter,
        } if command == "task" && parameter == "action"
    ));
}
