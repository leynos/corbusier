//! Shared test helpers for in-memory repository integration tests.

use corbusier::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{
        ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart, ToolCallPart,
        ToolResultPart,
    },
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::fixture;
use serde_json::json;
use std::io;
use tokio::runtime::Runtime;

/// Provides a tokio runtime for async operations in tests.
///
/// # Errors
///
/// Returns an error if the runtime cannot be created.
#[fixture]
pub fn runtime() -> io::Result<Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

/// Provides a fresh in-memory repository for each test.
#[fixture]
pub fn repo() -> InMemoryMessageRepository {
    InMemoryMessageRepository::new()
}

/// Provides a clock for message creation.
#[fixture]
pub fn clock() -> DefaultClock {
    DefaultClock
}

/// Provides a conversation ID for tests.
#[fixture]
pub fn conversation_id() -> ConversationId {
    ConversationId::new()
}

/// Stores conversation messages and returns them for verification.
///
/// # Errors
///
/// Returns an error if any message creation or store operation fails.
pub fn store_conversation_messages(
    rt: &Runtime,
    repo: &InMemoryMessageRepository,
    clock: &DefaultClock,
    conversation_id: ConversationId,
) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
    let user_message = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("What's in main.rs?"))],
        SequenceNumber::new(1),
        clock,
    )?;

    rt.block_on(repo.store(&user_message))?;

    let assistant_message = Message::new(
        conversation_id,
        Role::Assistant,
        vec![
            ContentPart::Text(TextPart::new("Let me check that file.")),
            ContentPart::ToolCall(ToolCallPart::new(
                "call-001",
                "read_file",
                json!({"path": "src/main.rs"}),
            )),
        ],
        SequenceNumber::new(2),
        clock,
    )?;

    rt.block_on(repo.store(&assistant_message))?;

    let tool_message = Message::new(
        conversation_id,
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-001",
            json!({"content": "fn main() { }"}),
        ))],
        SequenceNumber::new(3),
        clock,
    )?;

    rt.block_on(repo.store(&tool_message))?;

    let final_message = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new(
            "The file contains an empty main function.",
        ))],
        SequenceNumber::new(4),
        clock,
    )?;

    rt.block_on(repo.store(&final_message))?;

    Ok(vec![
        user_message,
        assistant_message,
        tool_message,
        final_message,
    ])
}

/// Verifies message ordering in a retrieved conversation.
pub fn verify_message_ordering(messages: &[Message]) {
    assert_eq!(messages.len(), 4);
    let expected_sequences = [1_u64, 2, 3, 4];
    for (message, expected) in messages.iter().zip(expected_sequences.iter()) {
        assert_eq!(message.sequence_number().value(), *expected);
    }
}

/// Verifies role preservation in retrieved messages.
pub fn verify_role_preservation(messages: &[Message]) {
    let expected_roles = [Role::User, Role::Assistant, Role::Tool, Role::Assistant];
    for (message, expected) in messages.iter().zip(expected_roles.iter()) {
        assert_eq!(message.role(), *expected);
    }
}
