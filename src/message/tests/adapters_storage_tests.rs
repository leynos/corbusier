//! Unit tests covering message adapter construction and storage semantics.

use super::adapters_test_support::{clock, ctx, make_message, repo};
use crate::context::RequestContext;
use crate::message::{
    adapters::memory::InMemoryMessageRepository, domain::ConversationId, error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;

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

#[rstest]
#[tokio::test]
async fn store_adds_message_to_repository(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_id = ConversationId::new();
    let message = make_message(conversation_id, 1, &clock)?;

    let result = repo.store(&ctx, &message).await;

    result.expect("failed to store message");
    assert_eq!(repo.len(), 1);
    assert!(!repo.is_empty());
    Ok(())
}

#[rstest]
#[tokio::test]
async fn store_rejects_duplicate_message_id(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_id = ConversationId::new();
    let message = make_message(conversation_id, 1, &clock)?;
    let id = message.id();

    repo.store(&ctx, &message).await.expect("first store");

    let result = repo.store(&ctx, &message).await;

    assert!(
        matches!(result, Err(RepositoryError::DuplicateMessage(duplicate_id)) if duplicate_id == message.id())
    );
    assert_eq!(repo.len(), 1);
    assert!(repo.exists(&ctx, id).await.expect("exists"));
    let found = repo.find_by_id(&ctx, id).await.expect("find_by_id");
    assert_eq!(found.expect("message should remain stored").id(), id);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn store_rejects_duplicate_sequence_number(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_id = ConversationId::new();
    let message1 = make_message(conversation_id, 1, &clock)?;
    let message2 = make_message(conversation_id, 1, &clock)?;

    repo.store(&ctx, &message1).await.expect("first store");

    let result = repo.store(&ctx, &message2).await;

    assert!(
        matches!(result, Err(RepositoryError::DuplicateSequence { conversation_id: cid, sequence })
            if cid == conversation_id && sequence.value() == 1)
    );
    Ok(())
}

#[rstest]
#[tokio::test]
async fn store_allows_different_message_ids(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_id = ConversationId::new();
    let message1 = make_message(conversation_id, 1, &clock)?;
    let message2 = make_message(conversation_id, 2, &clock)?;

    repo.store(&ctx, &message1).await.expect("store message 1");
    repo.store(&ctx, &message2).await.expect("store message 2");

    assert_eq!(repo.len(), 2);
    Ok(())
}
