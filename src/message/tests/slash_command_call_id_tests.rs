//! Unit tests for deterministic slash-command call identifiers.
//!
//! Call IDs are persisted in audit records, so these tests pin the rendered
//! form rather than only its prefix.

use std::sync::Arc;

use mockable::DefaultClock;
use rstest::{fixture, rstest};

use super::adapters_test_support::{clock, ctx, repo};
use crate::context::RequestContext;
use crate::message::{
    adapters::memory::{InMemoryMessageRepository, InMemorySlashCommandRegistry},
    domain::{
        ContentPart, ConversationId, Message, MessageMetadata, PlannedToolCall, Role,
        SequenceNumber, SlashCommandDefinition, TextPart, ToolCallTemplate,
    },
    ports::repository::MessageRepository,
    services::SlashCommandService,
};

/// Provides the default in-memory slash-command registry for call-ID tests.
#[fixture]
fn slash_command_service() -> SlashCommandService<InMemorySlashCommandRegistry> {
    SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()))
}

/// Stores a generated audit record and reads it back through the message port.
#[rstest]
#[tokio::test]
async fn call_id_survives_message_store_and_retrieval(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
    repo: InMemoryMessageRepository,
    ctx: RequestContext,
    clock: DefaultClock,
) {
    let execution = slash_command_service
        .execute("/task action=start issue=42")
        .expect("slash command should execute");
    let message = Message::builder(
        ConversationId::new(),
        Role::Assistant,
        SequenceNumber::new(1),
    )
    .with_content(ContentPart::Text(TextPart::new("Starting task")))
    .with_metadata(
        MessageMetadata::empty().with_tool_call_audits(execution.tool_call_audits().to_vec()),
    )
    .build(&clock)
    .expect("message should build");
    let message_id = message.id();

    repo.store(&ctx, &message)
        .await
        .expect("message should store");
    let stored_message = repo
        .find_by_id(&ctx, message_id)
        .await
        .expect("message lookup should succeed")
        .expect("stored message should be found");
    let audit = stored_message
        .metadata()
        .tool_call_audits
        .first()
        .expect("slash-command audit should be stored");

    assert_eq!(audit.call_id, "sc-0-81e30d6fd777c32f");
}

/// Pins the whole identifier for a fixed input to a digest computed outside
/// this crate (`sha256sum` over the canonical payload). Call IDs are persisted
/// in audit records, so the move off the local nibble table onto the shared
/// hex encoder must not shift them. Asserting only the `sc-0-` prefix would
/// pass even if the suffix collapsed to a constant.
#[rstest]
fn call_id_matches_externally_computed_digest(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
) {
    let execution = slash_command_service
        .execute("/task action=start issue=42")
        .expect("execution should succeed");

    let call_ids = execution
        .planned_tool_calls()
        .iter()
        .map(PlannedToolCall::call_id)
        .collect::<Vec<_>>();
    assert_eq!(call_ids, vec!["sc-0-81e30d6fd777c32f"]);
}

/// The suffix must track the hashed payload rather than being constant, so a
/// different parameter value has to produce a different digest.
#[rstest]
fn call_id_suffix_varies_with_input(
    slash_command_service: SlashCommandService<InMemorySlashCommandRegistry>,
) {
    let first = slash_command_service
        .execute("/task action=start issue=42")
        .expect("first execution should succeed");
    let second = slash_command_service
        .execute("/task action=start issue=43")
        .expect("second execution should succeed");

    assert_ne!(
        first
            .planned_tool_calls()
            .first()
            .map(PlannedToolCall::call_id),
        second
            .planned_tool_calls()
            .first()
            .map(PlannedToolCall::call_id),
    );
}

/// The index must feed the hashed payload, not merely decorate the prefix, so
/// two tool calls in one command differ in the digest as well as the `sc-<n>-`
/// segment.
#[rstest]
fn call_id_index_feeds_the_digest_not_just_the_prefix() {
    let definition = SlashCommandDefinition::new("twice", "Runs twice", "Twice")
        .with_tool_call(ToolCallTemplate::new("first_tool", r#"{"step": "one"}"#))
        .with_tool_call(ToolCallTemplate::new("first_tool", r#"{"step": "one"}"#));
    let registry =
        InMemorySlashCommandRegistry::with_commands([definition]).expect("registry should build");
    let service = SlashCommandService::new(Arc::new(registry));

    let execution = service.execute("/twice").expect("execution should succeed");

    let suffixes = execution
        .planned_tool_calls()
        .iter()
        .map(|tool_call| {
            tool_call
                .call_id()
                .rsplit_once('-')
                .map(|(_, suffix)| suffix.to_owned())
                .expect("call ID should carry a '-' separated suffix")
        })
        .collect::<Vec<_>>();

    assert_eq!(suffixes.len(), 2, "expected one call ID per tool call");
    assert_ne!(
        suffixes.first(),
        suffixes.last(),
        "identical tool calls at different indices must hash differently",
    );
}
