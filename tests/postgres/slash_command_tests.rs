//! `PostgreSQL` integration tests for slash-command execution metadata.

use std::sync::Arc;

use crate::postgres::cluster::BoxError;
use crate::postgres::helpers::{PreparedRepo, insert_conversation, prepared_repo};
use corbusier::message::{
    adapters::memory::InMemorySlashCommandRegistry,
    domain::{
        ContentPart, ConversationId, Message, MessageMetadata, Role, SequenceNumber, TextPart,
        ToolCallStatus,
    },
    ports::repository::MessageRepository,
    services::SlashCommandService,
};
use mockable::DefaultClock;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn slash_command_metadata_round_trip_postgres(
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let context = prepared_repo.await?;
    let repo = context.repo;
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let conversation_id = ConversationId::new();
    insert_conversation(context.cluster, context.temp_db.name(), conversation_id).await?;

    let execution = service.execute("/review action=sync include_summary=true")?;
    let deterministic_rerun = service.execute("/review action=sync include_summary=true")?;
    let (command_expansion, tool_call_audits) = execution.into_expansion_and_audits();
    let metadata = MessageMetadata::empty()
        .with_slash_command_expansion(command_expansion)
        .with_tool_call_audits(tool_call_audits);

    let message = Message::builder(conversation_id, Role::System, SequenceNumber::new(1))
        .with_content(ContentPart::Text(TextPart::new(
            "Slash command orchestration persisted",
        )))
        .with_metadata(metadata)
        .build(&DefaultClock)?;

    repo.store(&message).await?;

    let stored = repo
        .find_by_conversation(conversation_id)
        .await?
        .first()
        .cloned()
        .ok_or_else(|| std::io::Error::other("expected stored message"))?;

    let persisted_expansion = stored
        .metadata()
        .slash_command_expansion
        .as_ref()
        .ok_or_else(|| std::io::Error::other("expected slash command expansion"))?;
    assert_eq!(persisted_expansion.command, "/review");
    assert_eq!(
        persisted_expansion.parameters.get("action"),
        Some(&serde_json::Value::String("sync".to_owned()))
    );
    assert_eq!(
        persisted_expansion.parameters.get("include_summary"),
        Some(&serde_json::Value::Bool(true))
    );

    assert_eq!(stored.metadata().tool_call_audits.len(), 1);
    let first_audit = stored
        .metadata()
        .tool_call_audits
        .first()
        .ok_or_else(|| std::io::Error::other("expected a tool call audit"))?;
    assert_eq!(first_audit.tool_name, "review_service");
    assert_eq!(first_audit.status, ToolCallStatus::Queued);
    assert!(first_audit.call_id.starts_with("sc-0-"));
    assert!(!first_audit.call_id.is_empty());

    let deterministic_call_id = deterministic_rerun
        .tool_call_audits()
        .first()
        .ok_or_else(|| std::io::Error::other("expected rerun tool call audit"))?
        .call_id
        .clone();
    assert_eq!(first_audit.call_id, deterministic_call_id);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn slash_command_unknown_command_returns_error(
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let context = prepared_repo.await?;
    let repo = context.repo;
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let conversation_id = ConversationId::new();
    insert_conversation(context.cluster, context.temp_db.name(), conversation_id).await?;

    let error = service
        .execute("/nonexistent action=start")
        .expect_err("unknown command should be rejected");

    assert!(matches!(
        error,
        corbusier::message::domain::SlashCommandError::UnknownCommand(command)
        if command == "nonexistent"
    ));

    let stored_messages = repo.find_by_conversation(conversation_id).await?;
    assert!(stored_messages.is_empty());
    Ok(())
}
