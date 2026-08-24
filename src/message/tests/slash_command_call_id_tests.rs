//! Unit tests for deterministic slash-command call identifiers.
//!
//! Call IDs are persisted in audit records, so these tests pin the rendered
//! form rather than only its prefix.

use std::sync::Arc;

use rstest::{fixture, rstest};

use crate::message::{
    adapters::memory::InMemorySlashCommandRegistry,
    domain::{PlannedToolCall, SlashCommandDefinition, ToolCallTemplate},
    services::SlashCommandService,
};

#[fixture]
fn slash_command_service() -> SlashCommandService<InMemorySlashCommandRegistry> {
    SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()))
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
