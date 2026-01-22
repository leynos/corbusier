//! Uniqueness constraint tests for `PostgreSQL` message repository.

use crate::postgres::helpers::{
    PostgresCluster, clock, create_test_message, ensure_template, insert_conversation,
    postgres_cluster, setup_repository,
};
use corbusier::message::{
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn store_rejects_duplicate_message_id(
    clock: DefaultClock,
    postgres_cluster: PostgresCluster,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (temp_db, repo) = setup_repository(cluster).await.expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id)
        .await
        .expect("conversation insert");

    let message = create_test_message(&clock, conv_id, 1).expect("test message");
    let msg_id = message.id();

    repo.store(&message).await.expect("first store");

    let duplicate = Message::builder(conv_id, Role::User, SequenceNumber::new(2))
        .with_id(msg_id)
        .with_content(ContentPart::Text(TextPart::new("Different content")))
        .build(&clock)
        .expect("duplicate message");

    let result = repo.store(&duplicate).await;
    assert!(
        matches!(result, Err(RepositoryError::DuplicateMessage(id)) if id == msg_id),
        "Expected DuplicateMessage error, got: {result:?}"
    );
}

#[rstest]
#[tokio::test]
async fn store_rejects_duplicate_sequence_in_conversation(
    clock: DefaultClock,
    postgres_cluster: PostgresCluster,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (temp_db, repo) = setup_repository(cluster).await.expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id)
        .await
        .expect("conversation insert");

    let msg1 = create_test_message(&clock, conv_id, 1).expect("test message");

    repo.store(&msg1).await.expect("first store");

    let msg2 = create_test_message(&clock, conv_id, 1).expect("test message");

    let result = repo.store(&msg2).await;
    assert!(
        matches!(
            result,
            Err(RepositoryError::DuplicateSequence {
                conversation_id: c,
                sequence: s
            }) if c == conv_id && s.value() == 1
        ),
        "Expected DuplicateSequence error, got: {result:?}"
    );
}

#[rstest]
#[tokio::test]
async fn store_allows_same_sequence_in_different_conversations(
    clock: DefaultClock,
    postgres_cluster: PostgresCluster,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (temp_db, repo) = setup_repository(cluster).await.expect("repository setup");

    let conv1 = ConversationId::new();
    let conv2 = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv1)
        .await
        .expect("conversation insert");
    insert_conversation(cluster, temp_db.name(), conv2)
        .await
        .expect("conversation insert");

    let msg1 = create_test_message(&clock, conv1, 1).expect("test message");
    let msg2 = create_test_message(&clock, conv2, 1).expect("test message");

    repo.store(&msg1).await.expect("store in conv1");
    repo.store(&msg2).await.expect("store in conv2");

    assert!(repo.exists(msg1.id()).await.expect("exists check"));
    assert!(repo.exists(msg2.id()).await.expect("exists check"));
}
