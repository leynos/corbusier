//! Sequence number management tests for `PostgreSQL` message repository.

use crate::postgres::helpers::{
    PostgresCluster, clock, create_test_message, ensure_template, insert_conversation,
    postgres_cluster, setup_repository,
};
use corbusier::message::{domain::ConversationId, ports::repository::MessageRepository};
use mockable::DefaultClock;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_one_for_empty(postgres_cluster: PostgresCluster) {
    let cluster = postgres_cluster;
    ensure_template(cluster).await.expect("template setup");
    let (_temp_db, repo) = setup_repository(cluster).await.expect("repository setup");

    let conv_id = ConversationId::new();
    let next = repo
        .next_sequence_number(conv_id)
        .await
        .expect("next_sequence_number");

    assert_eq!(next.value(), 1);
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_max_plus_one(
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

    repo.store(&create_test_message(&clock, conv_id, 1).expect("test message"))
        .await
        .expect("store 1");
    repo.store(&create_test_message(&clock, conv_id, 2).expect("test message"))
        .await
        .expect("store 2");
    repo.store(&create_test_message(&clock, conv_id, 5).expect("test message"))
        .await
        .expect("store 5");

    let next = repo
        .next_sequence_number(conv_id)
        .await
        .expect("next_sequence_number");

    assert_eq!(next.value(), 6);
}
