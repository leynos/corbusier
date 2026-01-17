//! Sequence number tests for [`InMemoryMessageRepository`].
//!
//! Tests sequence number generation across multiple conversations.

use crate::in_memory::helpers::{clock, repo, runtime};
use corbusier::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{ContentPart, ConversationId, Message, Role, TextPart},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;
use std::io;
use tokio::runtime::Runtime;

/// Tests sequence number generation across multiple conversations.
#[rstest]
fn generation_across_conversations(
    runtime: io::Result<Runtime>,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
) {
    let rt = runtime.expect("runtime creation failed");
    let conv1 = ConversationId::new();
    let conv2 = ConversationId::new();

    let next1 = rt
        .block_on(repo.next_sequence_number(conv1))
        .expect("next_sequence_number failed");
    assert_eq!(next1.value(), 1);

    let msg1 = Message::new(
        conv1,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello conv1"))],
        next1,
        &clock,
    )
    .expect("message creation failed");
    rt.block_on(repo.store(&msg1)).expect("store failed");

    let next2 = rt
        .block_on(repo.next_sequence_number(conv1))
        .expect("next_sequence_number failed");
    assert_eq!(next2.value(), 2);

    let next_conv2 = rt
        .block_on(repo.next_sequence_number(conv2))
        .expect("next_sequence_number failed");
    assert_eq!(next_conv2.value(), 1);

    let msg2 = Message::new(
        conv2,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello conv2"))],
        next_conv2,
        &clock,
    )
    .expect("message creation failed");
    rt.block_on(repo.store(&msg2)).expect("store failed");

    let conv1_messages = rt
        .block_on(repo.find_by_conversation(conv1))
        .expect("find_by_conversation failed");
    let conv2_messages = rt
        .block_on(repo.find_by_conversation(conv2))
        .expect("find_by_conversation failed");

    assert_eq!(conv1_messages.len(), 1);
    assert_eq!(conv2_messages.len(), 1);
    assert_ne!(
        conv1_messages.first().expect("no messages in conv1").id(),
        conv2_messages.first().expect("no messages in conv2").id()
    );
}
