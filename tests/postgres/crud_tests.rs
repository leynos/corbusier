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
        if let Err(err) = self.guard.cleanup() {
            panic!("failed to cleanup CRUD test database: {err}");
        }
    }
}

#[fixture]
fn crud_context(postgres_cluster: PostgresCluster) -> CrudTestContext {
    let cluster = postgres_cluster;
    if let Err(err) = ensure_template(cluster) {
        panic!("failed to ensure postgres template: {err}");
    }
    let db_name = format!("test_crud_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = match setup_repository(cluster, &db_name) {
        Ok(repo) => repo,
        Err(err) => panic!("failed to setup postgres repository: {err}"),
    };
    let rt = match test_runtime() {
        Ok(rt) => rt,
        Err(err) => panic!("failed to create runtime: {err}"),
    };
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
    if let Err(err) = insert_conversation(context.cluster, &context.db_name, conv_id) {
        panic!("failed to insert conversation: {err}");
    }

    let message = match create_test_message(&clock, conv_id, 1) {
        Ok(message) => message,
        Err(err) => panic!("failed to create test message: {err}"),
    };
    let msg_id = message.id();

    if let Err(err) = context.rt.block_on(context.repo.store(&message)) {
        panic!("failed to store message: {err}");
    }

    let retrieved_opt = match context.rt.block_on(context.repo.find_by_id(msg_id)) {
        Ok(retrieved) => retrieved,
        Err(err) => panic!("failed to load message: {err}"),
    };
    let Some(retrieved) = retrieved_opt else {
        panic!("message should exist");
    };

    assert_eq!(retrieved.id(), msg_id);
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::User);
    assert_eq!(retrieved.sequence_number().value(), 1);

    context.cleanup();
}

#[rstest]
fn find_by_id_returns_none_for_missing(crud_context: CrudTestContext) {
    let context = crud_context;

    let result = match context
        .rt
        .block_on(context.repo.find_by_id(MessageId::new()))
    {
        Ok(result) => result,
        Err(err) => panic!("failed to load missing message: {err}"),
    };
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
    if let Err(err) = insert_conversation(context.cluster, &context.db_name, conv_id) {
        panic!("failed to insert conversation: {err}");
    }

    let msg3 = match create_test_message(&clock, conv_id, 3) {
        Ok(message) => message,
        Err(err) => panic!("failed to create message 3: {err}"),
    };
    let msg1 = match create_test_message(&clock, conv_id, 1) {
        Ok(message) => message,
        Err(err) => panic!("failed to create message 1: {err}"),
    };
    let msg2 = match create_test_message(&clock, conv_id, 2) {
        Ok(message) => message,
        Err(err) => panic!("failed to create message 2: {err}"),
    };

    if let Err(err) = context.rt.block_on(context.repo.store(&msg3)) {
        panic!("failed to store message 3: {err}");
    }
    if let Err(err) = context.rt.block_on(context.repo.store(&msg1)) {
        panic!("failed to store message 1: {err}");
    }
    if let Err(err) = context.rt.block_on(context.repo.store(&msg2)) {
        panic!("failed to store message 2: {err}");
    }

    let messages = match context
        .rt
        .block_on(context.repo.find_by_conversation(conv_id))
    {
        Ok(messages) => messages,
        Err(err) => panic!("failed to fetch messages: {err}"),
    };

    assert_eq!(messages.len(), 3);
    let sequence_numbers: Vec<_> = messages
        .iter()
        .map(|message| message.sequence_number().value())
        .collect();
    assert_eq!(sequence_numbers, vec![1, 2, 3]);

    context.cleanup();
}

#[rstest]
fn exists_returns_correct_status(clock: DefaultClock, crud_context: CrudTestContext) {
    let context = crud_context;

    let conv_id = ConversationId::new();
    if let Err(err) = insert_conversation(context.cluster, &context.db_name, conv_id) {
        panic!("failed to insert conversation: {err}");
    }

    let message = match create_test_message(&clock, conv_id, 1) {
        Ok(message) => message,
        Err(err) => panic!("failed to create message: {err}"),
    };
    let msg_id = message.id();

    let exists_before = match context.rt.block_on(context.repo.exists(msg_id)) {
        Ok(exists) => exists,
        Err(err) => panic!("failed to check message existence: {err}"),
    };
    assert!(!exists_before);

    if let Err(err) = context.rt.block_on(context.repo.store(&message)) {
        panic!("failed to store message: {err}");
    }
    let exists_after = match context.rt.block_on(context.repo.exists(msg_id)) {
        Ok(exists) => exists,
        Err(err) => panic!("failed to check message existence: {err}"),
    };
    assert!(exists_after);

    context.cleanup();
}
