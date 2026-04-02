//! Unit tests covering message adapter query semantics and shared state.

use super::adapters_test_support::{clock, ctx, make_message, repo};
use crate::context::RequestContext;
use crate::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{ConversationId, MessageId},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn find_by_id_returns_stored_message(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_id = ConversationId::new();
    let message = make_message(conversation_id, 1, &clock)?;
    let id = message.id();

    repo.store(&ctx, &message).await.expect("store");

    let result = repo.find_by_id(&ctx, id).await.expect("find_by_id");
    let found = result.expect("message should exist");
    assert_eq!(found.id(), id);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn find_by_id_returns_none_for_missing_id(
    repo: InMemoryMessageRepository,
    ctx: RequestContext,
) {
    let result = repo
        .find_by_id(&ctx, MessageId::new())
        .await
        .expect("find_by_id");
    assert!(result.is_none());
}

#[rstest]
#[tokio::test]
async fn find_by_conversation_returns_messages_in_order(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_id = ConversationId::new();
    let message3 = make_message(conversation_id, 3, &clock)?;
    let message1 = make_message(conversation_id, 1, &clock)?;
    let message2 = make_message(conversation_id, 2, &clock)?;

    repo.store(&ctx, &message3).await.expect("store 3");
    repo.store(&ctx, &message1).await.expect("store 1");
    repo.store(&ctx, &message2).await.expect("store 2");

    let messages = repo
        .find_by_conversation(&ctx, conversation_id)
        .await
        .expect("find_by_conversation");

    assert_eq!(messages.len(), 3);
    let seq_values: Vec<_> = messages
        .iter()
        .map(|message| message.sequence_number().value())
        .collect();
    assert_eq!(seq_values, vec![1, 2, 3]);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn find_by_conversation_filters_by_conversation(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_a = ConversationId::new();
    let conversation_b = ConversationId::new();

    let msg_a1 = make_message(conversation_a, 1, &clock)?;
    let msg_a2 = make_message(conversation_a, 2, &clock)?;
    let msg_b1 = make_message(conversation_b, 1, &clock)?;

    repo.store(&ctx, &msg_a1).await.expect("store a1");
    repo.store(&ctx, &msg_a2).await.expect("store a2");
    repo.store(&ctx, &msg_b1).await.expect("store b1");

    let messages_a = repo
        .find_by_conversation(&ctx, conversation_a)
        .await
        .expect("find conversation a");
    let messages_b = repo
        .find_by_conversation(&ctx, conversation_b)
        .await
        .expect("find conversation b");

    assert_eq!(messages_a.len(), 2);
    assert_eq!(messages_b.len(), 1);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn find_by_conversation_returns_empty_for_unknown_conversation(
    repo: InMemoryMessageRepository,
    ctx: RequestContext,
) {
    let messages = repo
        .find_by_conversation(&ctx, ConversationId::new())
        .await
        .expect("find_by_conversation");

    assert!(messages.is_empty());
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_1_for_empty_conversation(
    repo: InMemoryMessageRepository,
    ctx: RequestContext,
) {
    let conversation_id = ConversationId::new();

    let next = repo
        .next_sequence_number(&ctx, conversation_id)
        .await
        .expect("next_sequence_number");

    assert_eq!(next.value(), 1);
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_max_plus_one(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_id = ConversationId::new();

    let msg1 = make_message(conversation_id, 5, &clock)?;
    let msg2 = make_message(conversation_id, 10, &clock)?;

    repo.store(&ctx, &msg1).await.expect("store 1");
    repo.store(&ctx, &msg2).await.expect("store 2");

    let next = repo
        .next_sequence_number(&ctx, conversation_id)
        .await
        .expect("next_sequence_number");

    assert_eq!(next.value(), 11);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_is_per_conversation(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let conversation_a = ConversationId::new();
    let conversation_b = ConversationId::new();

    let msg_a = make_message(conversation_a, 100, &clock)?;
    repo.store(&ctx, &msg_a).await.expect("store a");

    let next_a = repo
        .next_sequence_number(&ctx, conversation_a)
        .await
        .expect("next a");
    let next_b = repo
        .next_sequence_number(&ctx, conversation_b)
        .await
        .expect("next b");

    assert_eq!(next_a.value(), 101);
    assert_eq!(next_b.value(), 1);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn exists_returns_true_for_stored_message(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let message = make_message(ConversationId::new(), 1, &clock)?;
    let id = message.id();

    repo.store(&ctx, &message).await.expect("store");

    let exists = repo.exists(&ctx, id).await.expect("exists");
    assert!(exists);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn exists_returns_false_for_missing_id(repo: InMemoryMessageRepository, ctx: RequestContext) {
    let exists = repo.exists(&ctx, MessageId::new()).await.expect("exists");
    assert!(!exists);
}

#[rstest]
#[tokio::test]
async fn len_tracks_message_count(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    assert_eq!(repo.len(), 0);

    let msg1 = make_message(ConversationId::new(), 1, &clock)?;
    let msg2 = make_message(ConversationId::new(), 2, &clock)?;

    repo.store(&ctx, &msg1).await.expect("store 1");
    assert_eq!(repo.len(), 1);

    repo.store(&ctx, &msg2).await.expect("store 2");
    assert_eq!(repo.len(), 2);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn is_empty_reflects_repository_state(
    repo: InMemoryMessageRepository,
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    assert!(repo.is_empty());

    let message = make_message(ConversationId::new(), 1, &clock)?;
    repo.store(&ctx, &message).await.expect("store");

    assert!(!repo.is_empty());
    Ok(())
}

#[rstest]
#[tokio::test]
async fn cloned_repository_shares_state(
    clock: DefaultClock,
    ctx: RequestContext,
) -> Result<(), crate::message::domain::MessageBuilderError> {
    let repo1 = InMemoryMessageRepository::new();
    let repo2 = repo1.clone();

    let message = make_message(ConversationId::new(), 1, &clock)?;

    repo1.store(&ctx, &message).await.expect("store via repo1");

    assert_eq!(repo2.len(), 1);
    let found = repo2.find_by_id(&ctx, message.id()).await.expect("find");
    assert!(found.is_some());
    Ok(())
}
