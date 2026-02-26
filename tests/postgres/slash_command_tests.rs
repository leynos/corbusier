//! `PostgreSQL` integration tests for slash-command execution metadata.

use std::sync::Arc;

use crate::postgres::cluster::BoxError;
use crate::postgres::helpers::{PreparedRepo, insert_conversation, prepared_repo};
use corbusier::message::{
    adapters::memory::InMemorySlashCommandRegistry,
    domain::{
        ContentPart, ConversationId, Message, MessageMetadata, Role, SequenceNumber, TextPart,
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
    let metadata = MessageMetadata::empty()
        .with_slash_command_expansion(execution.expansion)
        .with_tool_call_audits(execution.tool_call_audits);

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

    assert_eq!(
        stored
            .metadata()
            .slash_command_expansion
            .as_ref()
            .map(|expansion| expansion.command.as_str()),
        Some("/review")
    );
    assert_eq!(stored.metadata().tool_call_audits.len(), 1);
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
