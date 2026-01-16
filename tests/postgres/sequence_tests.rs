//! Sequence number management tests for `PostgreSQL` message repository.

use crate::postgres::helpers::{
    CleanupGuard, clock, create_test_message, ensure_template, insert_conversation,
    setup_repository, test_runtime,
};
use corbusier::message::{domain::ConversationId, ports::repository::MessageRepository};
use mockable::DefaultClock;
use pg_embedded_setup_unpriv::TestCluster;
use pg_embedded_setup_unpriv::test_support::shared_test_cluster;
use rstest::rstest;

#[rstest]
fn next_sequence_number_returns_one_for_empty(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_next_seq_empty_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    let rt = test_runtime();
    let next = rt
        .block_on(repo.next_sequence_number(conv_id))
        .expect("next_sequence_number");

    assert_eq!(next.value(), 1);
}

#[rstest]
fn next_sequence_number_returns_max_plus_one(
    clock: DefaultClock,
    shared_test_cluster: &'static TestCluster,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_next_seq_incr_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let rt = test_runtime();

    rt.block_on(repo.store(&create_test_message(&clock, conv_id, 1)))
        .expect("store 1");
    rt.block_on(repo.store(&create_test_message(&clock, conv_id, 2)))
        .expect("store 2");
    rt.block_on(repo.store(&create_test_message(&clock, conv_id, 5)))
        .expect("store 5");

    let next = rt
        .block_on(repo.next_sequence_number(conv_id))
        .expect("next_sequence_number");

    assert_eq!(next.value(), 6);
}
