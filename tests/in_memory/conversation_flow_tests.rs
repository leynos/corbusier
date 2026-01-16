//! Conversation flow tests for [`InMemoryMessageRepository`].
//!
//! Tests realistic conversation patterns including message ordering,
//! role preservation, and individual retrieval.

use crate::in_memory::helpers::{
    clock, conversation_id, repo, runtime, store_conversation_messages, verify_message_ordering,
    verify_role_preservation,
};
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
fn stores_messages_in_order(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    store_conversation_messages(&rt, &repo, &clock, conversation_id)?;

    let messages = rt.block_on(repo.find_by_conversation(conversation_id))?;

    verify_message_ordering(&messages);
    Ok(())
}

/// Tests that roles are preserved through storage and retrieval.
#[rstest]
fn preserves_roles(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    store_conversation_messages(&rt, &repo, &clock, conversation_id)?;

    let messages = rt.block_on(repo.find_by_conversation(conversation_id))?;

    verify_role_preservation(&messages);
    Ok(())
}

/// Tests individual message retrieval by ID.
#[rstest]
#[expect(
    clippy::indexing_slicing,
    reason = "Test accesses first element after store_conversation_messages returns 4 elements"
)]
fn allows_individual_retrieval(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    let stored = store_conversation_messages(&rt, &repo, &clock, conversation_id)?;
    let first_message = &stored[0];

    let retrieved = rt
        .block_on(repo.find_by_id(first_message.id()))?
        .expect("exists");

    assert_eq!(retrieved.id(), first_message.id());
    Ok(())
}

/// Tests that repository correctly handles concurrent-like access patterns.
#[rstest]
fn concurrent_access_pattern_with_cloned_repository(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
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
    rt.block_on(repo.store(&msg1))?;

    let msg2 = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("From clone"))],
        SequenceNumber::new(2),
        &clock,
    )?;
    rt.block_on(repo_clone.store(&msg2))?;

    let from_original = rt.block_on(repo.find_by_conversation(conversation_id))?;
    let from_clone = rt.block_on(repo_clone.find_by_conversation(conversation_id))?;

    assert_eq!(from_original.len(), 2);
    assert_eq!(from_clone.len(), 2);
    Ok(())
}
