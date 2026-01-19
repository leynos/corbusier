//! Basic CRUD operation tests for `PostgreSQL` message repository.

use crate::postgres::helpers::{
    CleanupGuard, PostgresCluster, clock, create_test_message, ensure_template,
    insert_conversation, postgres_cluster, setup_repository, test_runtime,
};
use corbusier::message::adapters::postgres::PostgresMessageRepository;
use corbusier::message::{
    domain::{ConversationId, MessageId, Role},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use tokio::runtime::Runtime;

struct CrudTestContext {
    cluster: PostgresCluster,
    db_name: String,
    guard: CleanupGuard<'static>,
    repo: PostgresMessageRepository,
    rt: Runtime,
}

impl CrudTestContext {
    fn cleanup(self) {
        drop(self.repo);
        self.guard.cleanup().expect("cleanup database");
    }
}

#[fixture]
fn crud_context(postgres_cluster: PostgresCluster) -> CrudTestContext {
    let cluster = postgres_cluster;
    ensure_template(cluster).expect("template setup");
    let db_name = format!("test_crud_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name).expect("repository setup");
    let rt = test_runtime().expect("tokio runtime");
    CrudTestContext {
        cluster,
        db_name,
        guard,
        repo,
        rt,
    }
}

#[rstest]
fn store_and_retrieve_message(clock: DefaultClock, crud_context: CrudTestContext) {
    let context = crud_context;

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)
        .expect("conversation insert");

    let message = create_test_message(&clock, conv_id, 1).expect("test message");
    let msg_id = message.id();

    context
        .rt
        .block_on(context.repo.store(&message))
        .expect("store should succeed");

    let retrieved = context
        .rt
        .block_on(context.repo.find_by_id(msg_id))
        .expect("find_by_id should succeed")
        .expect("message should exist");

    assert_eq!(retrieved.id(), msg_id);
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::User);
    assert_eq!(retrieved.sequence_number().value(), 1);

    context.cleanup();
}

#[rstest]
fn find_by_id_returns_none_for_missing(crud_context: CrudTestContext) {
    let context = crud_context;

    let result = context
        .rt
        .block_on(context.repo.find_by_id(MessageId::new()))
        .expect("query ok");
    assert!(result.is_none());

    context.cleanup();
}

#[rstest]
fn find_by_conversation_returns_ordered_messages(
    clock: DefaultClock,
    crud_context: CrudTestContext,
) {
    let context = crud_context;

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)
        .expect("conversation insert");

    let msg3 = create_test_message(&clock, conv_id, 3).expect("test message");
    let msg1 = create_test_message(&clock, conv_id, 1).expect("test message");
    let msg2 = create_test_message(&clock, conv_id, 2).expect("test message");

    context.rt.block_on(context.repo.store(&msg3)).expect("store msg3");
    context.rt.block_on(context.repo.store(&msg1)).expect("store msg1");
    context.rt.block_on(context.repo.store(&msg2)).expect("store msg2");

    let messages = context
        .rt
        .block_on(context.repo.find_by_conversation(conv_id))
        .expect("find_by_conversation");

    assert_eq!(messages.len(), 3);
    let [first, second, third] = messages.as_slice() else {
        panic!("Expected 3 messages, got {}", messages.len());
    };
    assert_eq!(first.sequence_number().value(), 1);
    assert_eq!(second.sequence_number().value(), 2);
    assert_eq!(third.sequence_number().value(), 3);

    context.cleanup();
}

#[rstest]
fn exists_returns_correct_status(clock: DefaultClock, crud_context: CrudTestContext) {
    let context = crud_context;

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)
        .expect("conversation insert");

    let message = create_test_message(&clock, conv_id, 1).expect("test message");
    let msg_id = message.id();

    assert!(!context
        .rt
        .block_on(context.repo.exists(msg_id))
        .expect("exists check"));

    context
        .rt
        .block_on(context.repo.store(&message))
        .expect("store");
    assert!(context
        .rt
        .block_on(context.repo.exists(msg_id))
        .expect("exists check"));

    context.cleanup();
}
