//! Basic CRUD operation tests for `PostgreSQL` message repository.

#![expect(
    clippy::indexing_slicing,
    reason = "Test code uses indexing after length checks"
)]

use crate::postgres::helpers::{
    CleanupGuard, clock, create_test_message, ensure_template, insert_conversation,
    setup_repository, test_runtime,
};
use corbusier::message::{
    domain::{ConversationId, MessageId, Role},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use pg_embedded_setup_unpriv::TestCluster;
use pg_embedded_setup_unpriv::test_support::shared_test_cluster;
use rstest::rstest;

#[rstest]
fn store_and_retrieve_message(clock: DefaultClock, shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_store_retrieve_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(&clock, conv_id, 1);
    let msg_id = message.id();

    let rt = test_runtime();

    rt.block_on(repo.store(&message))
        .expect("store should succeed");

    let retrieved = rt
        .block_on(repo.find_by_id(msg_id))
        .expect("find_by_id should succeed")
        .expect("message should exist");

    assert_eq!(retrieved.id(), msg_id);
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::User);
    assert_eq!(retrieved.sequence_number().value(), 1);
}

#[rstest]
fn find_by_id_returns_none_for_missing(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_find_none_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let rt = test_runtime();
    let result = rt
        .block_on(repo.find_by_id(MessageId::new()))
        .expect("query ok");
    assert!(result.is_none());
}

#[rstest]
fn find_by_conversation_returns_ordered_messages(
    clock: DefaultClock,
    shared_test_cluster: &'static TestCluster,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_find_conv_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let msg3 = create_test_message(&clock, conv_id, 3);
    let msg1 = create_test_message(&clock, conv_id, 1);
    let msg2 = create_test_message(&clock, conv_id, 2);

    let rt = test_runtime();
    rt.block_on(repo.store(&msg3)).expect("store msg3");
    rt.block_on(repo.store(&msg1)).expect("store msg1");
    rt.block_on(repo.store(&msg2)).expect("store msg2");

    let messages = rt
        .block_on(repo.find_by_conversation(conv_id))
        .expect("find_by_conversation");

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].sequence_number().value(), 1);
    assert_eq!(messages[1].sequence_number().value(), 2);
    assert_eq!(messages[2].sequence_number().value(), 3);
}

#[rstest]
fn exists_returns_correct_status(clock: DefaultClock, shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_exists_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(&clock, conv_id, 1);
    let msg_id = message.id();

    let rt = test_runtime();

    assert!(!rt.block_on(repo.exists(msg_id)).expect("exists check"));

    rt.block_on(repo.store(&message)).expect("store");
    assert!(rt.block_on(repo.exists(msg_id)).expect("exists check"));
}
