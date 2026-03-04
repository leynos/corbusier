//! Conversation flow tests for [`InMemoryMessageRepository`].
//!
//! Tests realistic conversation patterns including message ordering,
//! role preservation, and individual retrieval.

use crate::in_memory::helpers::{
    clock, conversation_id, ctx, repo, runtime, store_conversation_messages,
    verify_message_ordering, verify_role_preservation,
};
use corbusier::context::RequestContext;
use corbusier::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;
use std::io;
use tokio::runtime::Runtime;

/// Tests storing a complete conversation and verifying message ordering.
#[rstest]
#[expect(
    clippy::too_many_arguments,
    reason = "rstest fixture injection requires individual parameters"
)]
fn stores_messages_in_order(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
    ctx: RequestContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    store_conversation_messages(&rt, &repo, &clock, conversation_id, &ctx)?;

    let messages = rt.block_on(repo.find_by_conversation(&ctx, conversation_id))?;

    verify_message_ordering(&messages);
    Ok(())
}

/// Tests that roles are preserved through storage and retrieval.
#[rstest]
#[expect(
    clippy::too_many_arguments,
    reason = "rstest fixture injection requires individual parameters"
)]
fn preserves_roles(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
    ctx: RequestContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    store_conversation_messages(&rt, &repo, &clock, conversation_id, &ctx)?;

    let messages = rt.block_on(repo.find_by_conversation(&ctx, conversation_id))?;

    verify_role_preservation(&messages);
    Ok(())
}

/// Tests individual message retrieval by ID.
#[rstest]
#[expect(
    clippy::indexing_slicing,
    reason = "Test accesses first element after store_conversation_messages returns 4 elements"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "rstest fixture injection requires individual parameters"
)]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test uses assertions for verification while returning Result for error propagation"
)]
fn allows_individual_retrieval(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
    ctx: RequestContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    let stored = store_conversation_messages(&rt, &repo, &clock, conversation_id, &ctx)?;
    let first_message = &stored[0];

    let retrieved = rt
        .block_on(repo.find_by_id(&ctx, first_message.id()))?
        .expect("exists");

    assert_eq!(retrieved.id(), first_message.id());
    Ok(())
}

/// Tests that repository correctly handles concurrent-like access patterns.
#[rstest]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test uses assertions for verification while returning Result for error propagation"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "rstest fixture injection requires individual parameters"
)]
fn concurrent_access_pattern_with_cloned_repository(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
    ctx: RequestContext,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    let repo_clone = repo.clone();

    let msg1 = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("From original"))],
        SequenceNumber::new(1),
        &clock,
    )?;
    rt.block_on(repo.store(&ctx, &msg1))?;

    let msg2 = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("From clone"))],
        SequenceNumber::new(2),
        &clock,
    )?;
    rt.block_on(repo_clone.store(&ctx, &msg2))?;

    let from_original = rt.block_on(repo.find_by_conversation(&ctx, conversation_id))?;
    let from_clone = rt.block_on(repo_clone.find_by_conversation(&ctx, conversation_id))?;

    assert_eq!(from_original.len(), 2);
    assert_eq!(from_clone.len(), 2);
    Ok(())
}
