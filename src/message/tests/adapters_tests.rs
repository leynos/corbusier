//! Unit tests for message repository adapters.
//!
//! Tests the `InMemoryMessageRepository` implementation via the public
//! `MessageRepository` trait interface.

use crate::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{ContentPart, ConversationId, Message, MessageId, Role, SequenceNumber, TextPart},
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

#[fixture]
fn repo() -> InMemoryMessageRepository {
    InMemoryMessageRepository::new()
}

fn make_message(conversation_id: ConversationId, seq: u64, clock: &DefaultClock) -> Message {
    Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new(format!("Message {seq}")))],
        SequenceNumber::new(seq),
        clock,
    )
    .expect("valid message")
}

// ============================================================================
// InMemoryMessageRepository constructor tests
// ============================================================================

#[test]
fn in_memory_repository_new_creates_empty_repo() {
    let repo = InMemoryMessageRepository::new();
    assert!(repo.is_empty());
    assert_eq!(repo.len(), 0);
}

#[test]
fn in_memory_repository_default_creates_empty_repo() {
    let repo = InMemoryMessageRepository::default();
    assert!(repo.is_empty());
    assert_eq!(repo.len(), 0);
}

// ============================================================================
// store tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn store_adds_message_to_repository(repo: InMemoryMessageRepository, clock: DefaultClock) {
    let conversation_id = ConversationId::new();
    let message = make_message(conversation_id, 1, &clock);

    let result = repo.store(&message).await;

    assert!(result.is_ok());
    assert_eq!(repo.len(), 1);
    assert!(!repo.is_empty());
}

#[rstest]
#[tokio::test]
async fn store_rejects_duplicate_message_id(repo: InMemoryMessageRepository, clock: DefaultClock) {
    let conversation_id = ConversationId::new();
    let message = make_message(conversation_id, 1, &clock);

    // First store succeeds
    repo.store(&message).await.expect("first store");

    // Second store with same message fails
    let result = repo.store(&message).await;
    assert!(result.is_err());

    // Verify error is Database variant (from our DuplicateIdError)
    assert!(matches!(result, Err(RepositoryError::Database(_))));
}

#[rstest]
#[tokio::test]
async fn store_allows_different_message_ids(repo: InMemoryMessageRepository, clock: DefaultClock) {
    let conversation_id = ConversationId::new();
    let message1 = make_message(conversation_id, 1, &clock);
    let message2 = make_message(conversation_id, 2, &clock);

    repo.store(&message1).await.expect("store message 1");
    repo.store(&message2).await.expect("store message 2");

    assert_eq!(repo.len(), 2);
}

// ============================================================================
// find_by_id tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn find_by_id_returns_stored_message(repo: InMemoryMessageRepository, clock: DefaultClock) {
    let conversation_id = ConversationId::new();
    let message = make_message(conversation_id, 1, &clock);
    let id = message.id();

    repo.store(&message).await.expect("store");

    let result = repo.find_by_id(id).await.expect("find_by_id");
    let found = result.expect("message should exist");
    assert_eq!(found.id(), id);
}

#[rstest]
#[tokio::test]
async fn find_by_id_returns_none_for_missing_id(repo: InMemoryMessageRepository) {
    let result = repo.find_by_id(MessageId::new()).await.expect("find_by_id");
    assert!(result.is_none());
}

// ============================================================================
// find_by_conversation tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn find_by_conversation_returns_messages_in_order(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
) {
    let conversation_id = ConversationId::new();

    // Store messages in reverse order to test sorting
    let message3 = make_message(conversation_id, 3, &clock);
    let message1 = make_message(conversation_id, 1, &clock);
    let message2 = make_message(conversation_id, 2, &clock);

    repo.store(&message3).await.expect("store 3");
    repo.store(&message1).await.expect("store 1");
    repo.store(&message2).await.expect("store 2");

    let messages = repo
        .find_by_conversation(conversation_id)
        .await
        .expect("find_by_conversation");

    assert_eq!(messages.len(), 3);
    // Verify ordering by sequence number
    let seq_values: Vec<_> = messages
        .iter()
        .map(|m| m.sequence_number().value())
        .collect();
    assert_eq!(seq_values, vec![1, 2, 3]);
}

#[rstest]
#[tokio::test]
async fn find_by_conversation_filters_by_conversation(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
) {
    let conversation_a = ConversationId::new();
    let conversation_b = ConversationId::new();

    let msg_a1 = make_message(conversation_a, 1, &clock);
    let msg_a2 = make_message(conversation_a, 2, &clock);
    let msg_b1 = make_message(conversation_b, 1, &clock);

    repo.store(&msg_a1).await.expect("store a1");
    repo.store(&msg_a2).await.expect("store a2");
    repo.store(&msg_b1).await.expect("store b1");

    let messages_a = repo
        .find_by_conversation(conversation_a)
        .await
        .expect("find conversation a");
    let messages_b = repo
        .find_by_conversation(conversation_b)
        .await
        .expect("find conversation b");

    assert_eq!(messages_a.len(), 2);
    assert_eq!(messages_b.len(), 1);
}

#[rstest]
#[tokio::test]
async fn find_by_conversation_returns_empty_for_unknown_conversation(
    repo: InMemoryMessageRepository,
) {
    let messages = repo
        .find_by_conversation(ConversationId::new())
        .await
        .expect("find_by_conversation");

    assert!(messages.is_empty());
}

// ============================================================================
// next_sequence_number tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_1_for_empty_conversation(repo: InMemoryMessageRepository) {
    let conversation_id = ConversationId::new();

    let next = repo
        .next_sequence_number(conversation_id)
        .await
        .expect("next_sequence_number");

    assert_eq!(next.value(), 1);
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_max_plus_one(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
) {
    let conversation_id = ConversationId::new();

    let msg1 = make_message(conversation_id, 5, &clock);
    let msg2 = make_message(conversation_id, 10, &clock);

    repo.store(&msg1).await.expect("store 1");
    repo.store(&msg2).await.expect("store 2");

    let next = repo
        .next_sequence_number(conversation_id)
        .await
        .expect("next_sequence_number");

    assert_eq!(next.value(), 11);
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_is_per_conversation(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
) {
    let conversation_a = ConversationId::new();
    let conversation_b = ConversationId::new();

    let msg_a = make_message(conversation_a, 100, &clock);
    repo.store(&msg_a).await.expect("store a");

    let next_a = repo
        .next_sequence_number(conversation_a)
        .await
        .expect("next a");
    let next_b = repo
        .next_sequence_number(conversation_b)
        .await
        .expect("next b");

    assert_eq!(next_a.value(), 101);
    assert_eq!(next_b.value(), 1);
}

// ============================================================================
// exists tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn exists_returns_true_for_stored_message(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
) {
    let message = make_message(ConversationId::new(), 1, &clock);
    let id = message.id();

    repo.store(&message).await.expect("store");

    let exists = repo.exists(id).await.expect("exists");
    assert!(exists);
}

#[rstest]
#[tokio::test]
async fn exists_returns_false_for_missing_id(repo: InMemoryMessageRepository) {
    let exists = repo.exists(MessageId::new()).await.expect("exists");
    assert!(!exists);
}

// ============================================================================
// len and is_empty tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn len_tracks_message_count(repo: InMemoryMessageRepository, clock: DefaultClock) {
    assert_eq!(repo.len(), 0);

    let msg1 = make_message(ConversationId::new(), 1, &clock);
    let msg2 = make_message(ConversationId::new(), 2, &clock);

    repo.store(&msg1).await.expect("store 1");
    assert_eq!(repo.len(), 1);

    repo.store(&msg2).await.expect("store 2");
    assert_eq!(repo.len(), 2);
}

#[rstest]
#[tokio::test]
async fn is_empty_reflects_repository_state(repo: InMemoryMessageRepository, clock: DefaultClock) {
    assert!(repo.is_empty());

    let message = make_message(ConversationId::new(), 1, &clock);
    repo.store(&message).await.expect("store");

    assert!(!repo.is_empty());
}

// ============================================================================
// Clone/thread-safety tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn cloned_repository_shares_state(clock: DefaultClock) {
    let repo1 = InMemoryMessageRepository::new();
    let repo2 = repo1.clone();

    let message = make_message(ConversationId::new(), 1, &clock);

    repo1.store(&message).await.expect("store via repo1");

    // repo2 should see the message stored via repo1
    assert_eq!(repo2.len(), 1);
    let found = repo2.find_by_id(message.id()).await.expect("find");
    assert!(found.is_some());
}
