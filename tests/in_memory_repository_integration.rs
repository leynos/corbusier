//! Behavioural integration tests for [`InMemoryMessageRepository`].
//!
//! These tests exercise the in-memory repository in realistic higher-level
//! flows, verifying that it correctly implements the repository contract
//! when used in conversation tracking scenarios.

#![expect(
    clippy::expect_used,
    reason = "Test code uses expect for assertion clarity"
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
use rstest::{fixture, rstest};
use serde_json::json;
use tokio::runtime::Runtime;

// ============================================================================
// Fixtures
// ============================================================================

/// Provides a tokio runtime for async operations in tests.
#[fixture]
fn runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create test runtime")
}

/// Provides a fresh in-memory repository for each test.
#[fixture]
fn repo() -> InMemoryMessageRepository {
    InMemoryMessageRepository::new()
}

/// Provides a clock for message creation.
#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

/// Provides a conversation ID for tests.
#[fixture]
fn conversation_id() -> ConversationId {
    ConversationId::new()
}

// ============================================================================
// Conversation Flow Helpers
// ============================================================================

/// Stores conversation messages and returns them for verification.
fn store_conversation_messages(
    rt: &Runtime,
    repo: &InMemoryMessageRepository,
    clock: &DefaultClock,
    conversation_id: ConversationId,
) -> Vec<Message> {
    let user_message = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("What's in main.rs?"))],
        SequenceNumber::new(1),
        clock,
    )
    .expect("user message");

    rt.block_on(repo.store(&user_message)).expect("store user");

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
    )
    .expect("assistant message");

    rt.block_on(repo.store(&assistant_message))
        .expect("store assistant");

    let tool_message = Message::new(
        conversation_id,
        Role::Tool,
        vec![ContentPart::ToolResult(ToolResultPart::success(
            "call-001",
            json!({"content": "fn main() { }"}),
        ))],
        SequenceNumber::new(3),
        clock,
    )
    .expect("tool message");

    rt.block_on(repo.store(&tool_message)).expect("store tool");

    let final_message = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new(
            "The file contains an empty main function.",
        ))],
        SequenceNumber::new(4),
        clock,
    )
    .expect("final message");

    rt.block_on(repo.store(&final_message))
        .expect("store final");

    vec![user_message, assistant_message, tool_message, final_message]
}

/// Verifies message ordering in a retrieved conversation.
#[expect(clippy::indexing_slicing, reason = "Test helper after length check")]
fn verify_message_ordering(messages: &[Message]) {
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0].sequence_number().value(), 1);
    assert_eq!(messages[1].sequence_number().value(), 2);
    assert_eq!(messages[2].sequence_number().value(), 3);
    assert_eq!(messages[3].sequence_number().value(), 4);
}

/// Verifies role preservation in retrieved messages.
#[expect(clippy::indexing_slicing, reason = "Test helper after length check")]
fn verify_role_preservation(messages: &[Message]) {
    assert_eq!(messages[0].role(), Role::User);
    assert_eq!(messages[1].role(), Role::Assistant);
    assert_eq!(messages[2].role(), Role::Tool);
    assert_eq!(messages[3].role(), Role::Assistant);
}

// ============================================================================
// Conversation Flow Tests
// ============================================================================

/// Tests storing a complete conversation and verifying message ordering.
#[rstest]
fn conversation_flow_stores_messages_in_order(
    runtime: Runtime,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) {
    store_conversation_messages(&runtime, &repo, &clock, conversation_id);

    let messages = runtime
        .block_on(repo.find_by_conversation(conversation_id))
        .expect("find conversation");

    verify_message_ordering(&messages);
}

/// Tests that roles are preserved through storage and retrieval.
#[rstest]
fn conversation_flow_preserves_roles(
    runtime: Runtime,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) {
    store_conversation_messages(&runtime, &repo, &clock, conversation_id);

    let messages = runtime
        .block_on(repo.find_by_conversation(conversation_id))
        .expect("find conversation");

    verify_role_preservation(&messages);
}

/// Tests individual message retrieval by ID.
#[rstest]
#[expect(
    clippy::indexing_slicing,
    reason = "Test accesses first element after store_conversation_messages returns 4 elements"
)]
fn conversation_flow_allows_individual_retrieval(
    runtime: Runtime,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) {
    let stored = store_conversation_messages(&runtime, &repo, &clock, conversation_id);
    let first_message = &stored[0];

    let retrieved = runtime
        .block_on(repo.find_by_id(first_message.id()))
        .expect("find by id")
        .expect("exists");

    assert_eq!(retrieved.id(), first_message.id());
}

// ============================================================================
// Sequence Number Tests
// ============================================================================

/// Tests sequence number generation across multiple conversations.
#[rstest]
#[expect(
    clippy::indexing_slicing,
    reason = "Test verifies length before access"
)]
fn sequence_generation_across_conversations(
    runtime: Runtime,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
) {
    let conv1 = ConversationId::new();
    let conv2 = ConversationId::new();

    let next1 = runtime
        .block_on(repo.next_sequence_number(conv1))
        .expect("next");
    assert_eq!(next1.value(), 1);

    let msg1 = Message::new(
        conv1,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello conv1"))],
        next1,
        &clock,
    )
    .expect("msg1");
    runtime.block_on(repo.store(&msg1)).expect("store");

    let next2 = runtime
        .block_on(repo.next_sequence_number(conv1))
        .expect("next");
    assert_eq!(next2.value(), 2);

    let next_conv2 = runtime
        .block_on(repo.next_sequence_number(conv2))
        .expect("next");
    assert_eq!(next_conv2.value(), 1);

    let msg2 = Message::new(
        conv2,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Hello conv2"))],
        next_conv2,
        &clock,
    )
    .expect("msg2");
    runtime.block_on(repo.store(&msg2)).expect("store");

    let conv1_messages = runtime
        .block_on(repo.find_by_conversation(conv1))
        .expect("find conv1");
    let conv2_messages = runtime
        .block_on(repo.find_by_conversation(conv2))
        .expect("find conv2");

    assert_eq!(conv1_messages.len(), 1);
    assert_eq!(conv2_messages.len(), 1);
    assert_ne!(conv1_messages[0].id(), conv2_messages[0].id());
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

/// Tests that repository correctly handles concurrent-like access patterns.
#[rstest]
fn concurrent_access_pattern_with_cloned_repository(
    runtime: Runtime,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) {
    let repo_clone = repo.clone();

    let msg1 = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("From original"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg1");
    runtime
        .block_on(repo.store(&msg1))
        .expect("store via original");

    let msg2 = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("From clone"))],
        SequenceNumber::new(2),
        &clock,
    )
    .expect("msg2");
    runtime
        .block_on(repo_clone.store(&msg2))
        .expect("store via clone");

    let from_original = runtime
        .block_on(repo.find_by_conversation(conversation_id))
        .expect("find via original");
    let from_clone = runtime
        .block_on(repo_clone.find_by_conversation(conversation_id))
        .expect("find via clone");

    assert_eq!(from_original.len(), 2);
    assert_eq!(from_clone.len(), 2);
}

// ============================================================================
// Duplicate Detection Tests
// ============================================================================

/// Tests that duplicate message IDs are rejected.
#[rstest]
fn duplicate_message_id_rejected(
    runtime: Runtime,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) {
    let msg = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Original message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg");

    runtime.block_on(repo.store(&msg)).expect("first store");

    let dup_id_msg = Message::builder(conversation_id, Role::User, SequenceNumber::new(2))
        .with_id(msg.id())
        .with_content(ContentPart::Text(TextPart::new("Different content")))
        .build(&clock)
        .expect("dup id msg");

    let result = runtime.block_on(repo.store(&dup_id_msg));
    assert!(
        matches!(result, Err(RepositoryError::DuplicateMessage(id)) if id == msg.id()),
        "Should reject duplicate message ID"
    );
}

/// Tests that duplicate sequence numbers in the same conversation are rejected.
#[rstest]
fn duplicate_sequence_in_conversation_rejected(
    runtime: Runtime,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) {
    let msg = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Original message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg");

    runtime.block_on(repo.store(&msg)).expect("first store");

    let dup_seq_msg = Message::new(
        conversation_id,
        Role::Assistant,
        vec![ContentPart::Text(TextPart::new("Response"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("dup seq msg");

    let result = runtime.block_on(repo.store(&dup_seq_msg));
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

// ============================================================================
// Exists Check Tests
// ============================================================================

/// Tests exists check in decision flow.
#[rstest]
fn exists_check_for_idempotent_operations(
    runtime: Runtime,
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    conversation_id: ConversationId,
) {
    let msg = Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("msg");

    let exists_before = runtime
        .block_on(repo.exists(msg.id()))
        .expect("exists check");
    assert!(!exists_before, "Should not exist before store");

    if !exists_before {
        runtime.block_on(repo.store(&msg)).expect("store");
    }

    let exists_after = runtime
        .block_on(repo.exists(msg.id()))
        .expect("exists check");
    assert!(exists_after, "Should exist after store");
}
