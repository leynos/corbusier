//! Sequence number management tests for `PostgreSQL` message repository.

use crate::postgres::helpers::{
    CleanupGuard, PostgresCluster, clock, create_test_message, ensure_template,
    insert_conversation, postgres_cluster, setup_repository, test_runtime,
};
use corbusier::message::{domain::ConversationId, ports::repository::MessageRepository};
use mockable::DefaultClock;
use rstest::rstest;

#[rstest]
fn next_sequence_number_returns_one_for_empty(postgres_cluster: PostgresCluster) {
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let db_name = format!("test_next_seq_empty_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    let rt = test_runtime().expect("tokio runtime");
    let next = rt
        .block_on(repo.next_sequence_number(conv_id))
        .expect("next_sequence_number");

    assert_eq!(next.value(), 1);

    drop(repo);

    guard.cleanup().expect("cleanup database");
}

#[rstest]
fn next_sequence_number_returns_max_plus_one(
    clock: DefaultClock,
    postgres_cluster: PostgresCluster,
) {
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let db_name = format!("test_next_seq_incr_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(cluster, &db_name, conv_id).expect("conversation insert");

    let rt = test_runtime().expect("tokio runtime");

    rt.block_on(repo.store(&create_test_message(&clock, conv_id, 1).expect("test message")))
        .expect("store 1");
    rt.block_on(repo.store(&create_test_message(&clock, conv_id, 2).expect("test message")))
        .expect("store 2");
    rt.block_on(repo.store(&create_test_message(&clock, conv_id, 5).expect("test message")))
        .expect("store 5");

    let next = rt
        .block_on(repo.next_sequence_number(conv_id))
        .expect("next_sequence_number");

    assert_eq!(next.value(), 6);

    drop(repo);

    guard.cleanup().expect("cleanup database");
}
