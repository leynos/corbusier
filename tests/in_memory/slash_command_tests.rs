//! In-memory integration tests for slash-command orchestration.

use std::error::Error;
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

#[rstest]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Assertions keep integration failure output concise in Result-based tests"
)]
fn slash_command_execution_metadata_round_trip_in_memory(
    runtime: std::io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let rt = runtime?;
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let execution = service.execute("/task action=start issue=42")?;
    let (command_expansion, tool_call_audits) = execution.into_expansion_and_audits();
    let metadata = MessageMetadata::empty()
        .with_slash_command_expansion(command_expansion)
        .with_tool_call_audits(tool_call_audits);

    let message = Message::builder(conversation_id, Role::System, SequenceNumber::new(1))
        .with_content(ContentPart::Text(TextPart::new(
            "Slash command planned for execution",
        )))
        .with_metadata(metadata)
        .build(&clock)?;

    rt.block_on(repo.store(&message))?;

    let persisted = rt
        .block_on(repo.find_by_conversation(conversation_id))?
        .first()
        .cloned()
        .ok_or_else(|| std::io::Error::other("expected persisted message"))?;

    assert_eq!(
        persisted
            .metadata()
            .slash_command_expansion
            .as_ref()
            .map(|persisted_expansion| persisted_expansion.command.as_str()),
        Some("/task")
    );
    assert_eq!(
        persisted
            .metadata()
            .slash_command_expansion
            .as_ref()
            .and_then(|persisted_expansion| {
                persisted_expansion
                    .parameters
                    .get("action")
                    .and_then(serde_json::Value::as_str)
            }),
        Some("start")
    );
    assert_eq!(
        persisted
            .metadata()
            .slash_command_expansion
            .as_ref()
            .and_then(|persisted_expansion| {
                persisted_expansion
                    .parameters
                    .get("issue")
                    .and_then(serde_json::Value::as_str)
            }),
        Some("42")
    );
    assert_eq!(persisted.metadata().tool_call_audits.len(), 1);
    let first_audit = persisted
        .metadata()
        .tool_call_audits
        .first()
        .ok_or_else(|| std::io::Error::other("expected tool call audit"))?;
    assert_eq!(first_audit.tool_name, "task_service");
    assert_eq!(first_audit.status, ToolCallStatus::Queued);
    assert!(first_audit.call_id.starts_with("sc-0-"));
    Ok(())
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
