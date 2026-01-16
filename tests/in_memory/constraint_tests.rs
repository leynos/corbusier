//! Constraint tests for [`InMemoryMessageRepository`].
//!
//! Tests duplicate detection and exists checks.

use crate::in_memory::helpers::{clock, conversation_id, repo, runtime};
use corbusier::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;
use std::io;
use tokio::runtime::Runtime;

/// Tests that duplicate message IDs are rejected.
#[rstest]
fn duplicate_message_id_rejected(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    let msg = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Original message"))],
        SequenceNumber::new(1),
        &clock,
    )?;

    rt.block_on(repo.store(&msg))?;

    let dup_id_msg = Message::builder(conversation_id, Role::User, SequenceNumber::new(2))
        .with_id(msg.id())
        .with_content(ContentPart::Text(TextPart::new("Different content")))
        .build(&clock)?;

    let result = rt.block_on(repo.store(&dup_id_msg));
    assert!(
        matches!(result, Err(RepositoryError::DuplicateMessage(id)) if id == msg.id()),
        "Should reject duplicate message ID"
    );
    Ok(())
}

/// Tests that duplicate sequence numbers in the same conversation are rejected.
#[rstest]
fn duplicate_sequence_in_conversation_rejected(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    let msg = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Original message"))],
        SequenceNumber::new(1),
        &clock,
    )?;

    rt.block_on(repo.store(&msg))?;

    let dup_seq_msg = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Response"))],
        SequenceNumber::new(1),
        &clock,
    )?;

    let result = rt.block_on(repo.store(&dup_seq_msg));
    assert!(
        matches!(
            result,
            Err(RepositoryError::DuplicateSequence {
                conversation_id: cid,
                sequence: seq
            }) if cid == conversation_id && seq.value() == 1
        ),
        "Should reject duplicate sequence number in same conversation"
    );
    Ok(())
}

/// Tests exists check in decision flow.
#[rstest]
fn exists_check_for_idempotent_operations(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rt = runtime?;
    let msg = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Message"))],
        SequenceNumber::new(1),
        &clock,
    )?;

    let exists_before = rt.block_on(repo.exists(msg.id()))?;
    assert!(!exists_before, "Should not exist before store");

    if !exists_before {
        rt.block_on(repo.store(&msg))?;
    }

    let exists_after = rt.block_on(repo.exists(msg.id()))?;
    assert!(exists_after, "Should exist after store");
    Ok(())
}
