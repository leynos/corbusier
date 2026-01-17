//! Uniqueness constraint tests for `PostgreSQL` message repository.

use crate::postgres::helpers::{
    CleanupGuard, PostgresCluster, clock, create_test_message, ensure_template,
    insert_conversation, postgres_cluster, setup_repository, test_runtime,
};
use corbusier::message::{
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::rstest;

#[rstest]
fn store_rejects_duplicate_message_id(clock: DefaultClock, postgres_cluster: PostgresCluster) {
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let db_name = format!("test_dup_msg_id_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, &db_name, conv_id).expect("conversation insert");

    let message = create_test_message(&clock, conv_id, 1).expect("test message");
    let msg_id = message.id();

    let rt = test_runtime().expect("tokio runtime");

    rt.block_on(repo.store(&message)).expect("first store");

    let duplicate = Message::builder(conv_id, Role::User, SequenceNumber::new(2))
        .with_id(msg_id)
        .with_content(ContentPart::Text(TextPart::new("Different content")))
        .build(&clock)
        .expect("duplicate message");

    let result = rt.block_on(repo.store(&duplicate));
    assert!(
        matches!(result, Err(RepositoryError::DuplicateMessage(id)) if id == msg_id),
        "Expected DuplicateMessage error, got: {result:?}"
    );

    drop(repo);

    guard.cleanup().expect("cleanup database");
}

#[rstest]
fn store_rejects_duplicate_sequence_in_conversation(
    clock: DefaultClock,
    postgres_cluster: PostgresCluster,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let db_name = format!("test_dup_seq_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, &db_name, conv_id).expect("conversation insert");

    let msg1 = create_test_message(&clock, conv_id, 1).expect("test message");

    let rt = test_runtime().expect("tokio runtime");
    rt.block_on(repo.store(&msg1)).expect("first store");

    let msg2 = create_test_message(&clock, conv_id, 1).expect("test message");

    let result = rt.block_on(repo.store(&msg2));
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

    drop(repo);

    guard.cleanup().expect("cleanup database");
}

#[rstest]
fn store_allows_same_sequence_in_different_conversations(
    clock: DefaultClock,
    postgres_cluster: PostgresCluster,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let db_name = format!("test_diff_conv_seq_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repository setup");

    let conv1 = ConversationId::new();
    let conv2 = ConversationId::new();
    insert_conversation(cluster, &db_name, conv1).expect("conversation insert");
    insert_conversation(cluster, &db_name, conv2).expect("conversation insert");

    let msg1 = create_test_message(&clock, conv1, 1).expect("test message");
    let msg2 = create_test_message(&clock, conv2, 1).expect("test message");

    let rt = test_runtime().expect("tokio runtime");

    rt.block_on(repo.store(&msg1)).expect("store in conv1");
    rt.block_on(repo.store(&msg2)).expect("store in conv2");

    assert!(rt.block_on(repo.exists(msg1.id())).expect("exists check"));
    assert!(rt.block_on(repo.exists(msg2.id())).expect("exists check"));

    drop(repo);

    guard.cleanup().expect("cleanup database");
}
