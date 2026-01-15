//! Behavioural integration tests for [`InMemoryMessageRepository`].
//!
//! These tests exercise the in-memory repository in realistic higher-level
//! flows, verifying that it correctly implements the repository contract
//! when used in conversation tracking scenarios.

#![expect(
    clippy::expect_used,
    reason = "Test code uses expect for assertion clarity"
)]
#![expect(
    clippy::indexing_slicing,
    reason = "Test code uses indexing after length checks"
)]
#![expect(
    clippy::cognitive_complexity,
    reason = "Test functions may have higher complexity for full scenario coverage"
)]
#![expect(
    clippy::shadow_unrelated,
    reason = "Test code reuses variable names for clarity in sequential assertions"
)]

use corbusier::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{
        ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart, ToolCallPart,
        ToolResultPart,
    },
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use serde_json::json;
use tokio::runtime::Runtime;

/// Creates a tokio runtime for async operations in tests.
fn test_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create test runtime")
}

// ============================================================================
// Conversation Flow Tests (Comment 10)
// ============================================================================

/// Simulates a complete conversation flow storing and retrieving messages
/// through the repository, verifying correct ordering and retrieval.
#[test]
fn complete_conversation_flow_through_repository() {
    let rt = test_runtime();
    let repo = InMemoryMessageRepository::new();
    let clock = DefaultClock;
    let conversation_id = ConversationId::new();

    // User asks a question (sequence 1)
    let user_message = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("What's in main.rs?"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("user message");

    rt.block_on(repo.store(&user_message)).expect("store user");

    // Assistant responds with tool call (sequence 2)
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
        &clock,
    )
    .expect("assistant message");

    rt.block_on(repo.store(&assistant_message))
        .expect("store assistant");

    // Tool result (sequence 3)
    let tool_message = Message::new(
        conversation_id,
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-001",
            json!({"content": "fn main() { }"}),
        ))],
        SequenceNumber::new(3),
        &clock,
    )
    .expect("tool message");

    rt.block_on(repo.store(&tool_message)).expect("store tool");

    // Final assistant response (sequence 4)
    let final_message = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new(
            "The file contains an empty main function.",
        ))],
        SequenceNumber::new(4),
        &clock,
    )
    .expect("final message");

    rt.block_on(repo.store(&final_message))
        .expect("store final");

    // Retrieve entire conversation
    let messages = rt
        .block_on(repo.find_by_conversation(conversation_id))
        .expect("find conversation");

    // Verify ordering
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0].sequence_number().value(), 1);
    assert_eq!(messages[1].sequence_number().value(), 2);
    assert_eq!(messages[2].sequence_number().value(), 3);
    assert_eq!(messages[3].sequence_number().value(), 4);

    // Verify roles preserved
    assert_eq!(messages[0].role(), Role::User);
    assert_eq!(messages[1].role(), Role::Assistant);
    assert_eq!(messages[2].role(), Role::Tool);
    assert_eq!(messages[3].role(), Role::Assistant);

    // Verify individual retrieval
    let retrieved = rt
        .block_on(repo.find_by_id(user_message.id()))
        .expect("find by id")
        .expect("exists");
    assert_eq!(retrieved.id(), user_message.id());
}

/// Tests sequence number generation across multiple conversations.
#[test]
fn sequence_generation_across_conversations() {
    let rt = test_runtime();
    let repo = InMemoryMessageRepository::new();
    let clock = DefaultClock;

    let conv1 = ConversationId::new();
    let conv2 = ConversationId::new();

    // First conversation gets sequence 1, 2
    let next1 = rt.block_on(repo.next_sequence_number(conv1)).expect("next");
    assert_eq!(next1.value(), 1);

    let msg1 = Message::new(
        conv1,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello conv1"))],
        next1,
        &clock,
    )
    .expect("msg1");
    rt.block_on(repo.store(&msg1)).expect("store");

    let next2 = rt.block_on(repo.next_sequence_number(conv1)).expect("next");
    assert_eq!(next2.value(), 2);

    // Second conversation independently starts at 1
    let next_conv2 = rt.block_on(repo.next_sequence_number(conv2)).expect("next");
    assert_eq!(next_conv2.value(), 1);

    let msg2 = Message::new(
        conv2,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello conv2"))],
        next_conv2,
        &clock,
    )
    .expect("msg2");
    rt.block_on(repo.store(&msg2)).expect("store");

    // Verify conversations are isolated
    let conv1_messages = rt
        .block_on(repo.find_by_conversation(conv1))
        .expect("find conv1");
    let conv2_messages = rt
        .block_on(repo.find_by_conversation(conv2))
        .expect("find conv2");

    assert_eq!(conv1_messages.len(), 1);
    assert_eq!(conv2_messages.len(), 1);
    assert_ne!(conv1_messages[0].id(), conv2_messages[0].id());
}

/// Tests that repository correctly handles concurrent-like access patterns.
#[test]
fn concurrent_access_pattern_with_cloned_repository() {
    let rt = test_runtime();
    let repo = InMemoryMessageRepository::new();
    let clock = DefaultClock;
    let conversation_id = ConversationId::new();

    // Clone repository (simulating shared state across service boundaries)
    let repo_clone = repo.clone();

    // Store via original
    let msg1 = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("From original"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg1");
    rt.block_on(repo.store(&msg1)).expect("store via original");

    // Store via clone
    let msg2 = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("From clone"))],
        SequenceNumber::new(2),
        &clock,
    )
    .expect("msg2");
    rt.block_on(repo_clone.store(&msg2))
        .expect("store via clone");

    // Both should see all messages
    let from_original = rt
        .block_on(repo.find_by_conversation(conversation_id))
        .expect("find via original");
    let from_clone = rt
        .block_on(repo_clone.find_by_conversation(conversation_id))
        .expect("find via clone");

    assert_eq!(from_original.len(), 2);
    assert_eq!(from_clone.len(), 2);
}

/// Tests duplicate detection in realistic insert patterns.
#[test]
fn duplicate_detection_in_flow() {
    let rt = test_runtime();
    let repo = InMemoryMessageRepository::new();
    let clock = DefaultClock;
    let conversation_id = ConversationId::new();

    let msg = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Original message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg");

    // First store succeeds
    rt.block_on(repo.store(&msg)).expect("first store");

    // Duplicate ID rejected
    let dup_id_msg = Message::builder(conversation_id, Role::User, SequenceNumber::new(2))
        .with_id(msg.id())
        .with_content(ContentPart::Text(TextPart::new("Different content")))
        .build(&clock)
        .expect("dup id msg");

    let result = rt.block_on(repo.store(&dup_id_msg));
    assert!(
        matches!(result, Err(RepositoryError::DuplicateMessage(id)) if id == msg.id()),
        "Should reject duplicate message ID"
    );

    // Duplicate sequence in same conversation rejected
    let dup_seq_msg = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Response"))],
        SequenceNumber::new(1), // Same sequence as original
        &clock,
    )
    .expect("dup seq msg");

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
}

/// Tests exists check in decision flow.
#[test]
fn exists_check_for_idempotent_operations() {
    let rt = test_runtime();
    let repo = InMemoryMessageRepository::new();
    let clock = DefaultClock;
    let conversation_id = ConversationId::new();

    let msg = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg");

    // Check before store
    let exists_before = rt.block_on(repo.exists(msg.id())).expect("exists check");
    assert!(!exists_before, "Should not exist before store");

    // Idempotent store pattern: check then store
    if !exists_before {
        rt.block_on(repo.store(&msg)).expect("store");
    }

    // Check after store
    let exists_after = rt.block_on(repo.exists(msg.id())).expect("exists check");
    assert!(exists_after, "Should exist after store");
}
