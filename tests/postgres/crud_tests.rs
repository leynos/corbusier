//! Basic CRUD operation tests for `PostgreSQL` message repository.

use crate::postgres::cluster::BoxError;
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
    fn cleanup(self) -> Result<(), BoxError> {
        drop(self.repo);
        self.guard.cleanup()
    }
}

#[fixture]
fn crud_context(postgres_cluster: PostgresCluster) -> Result<CrudTestContext, BoxError> {
    let cluster = postgres_cluster;
    ensure_template(cluster)?;
    let db_name = format!("test_crud_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name)?;
    let rt = test_runtime()?;
    Ok(CrudTestContext {
        cluster,
        db_name,
        guard,
        repo,
        rt,
    })
}

#[rstest]
fn store_and_retrieve_message(
    clock: DefaultClock,
    crud_context: Result<CrudTestContext, BoxError>,
) {
    let context = crud_context.expect("failed to create CRUD test context");

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)
        .expect("failed to insert conversation");

    let message = create_test_message(&clock, conv_id, 1).expect("failed to create test message");
    let msg_id = message.id();

    context
        .rt
        .block_on(context.repo.store(&message))
        .expect("failed to store message");

    let retrieved_opt = context
        .rt
        .block_on(context.repo.find_by_id(msg_id))
        .expect("failed to load message");
    let retrieved = retrieved_opt.expect("message should exist");

    assert_eq!(retrieved.id(), msg_id);
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::User);
    assert_eq!(retrieved.sequence_number().value(), 1);

    context
        .cleanup()
        .expect("failed to cleanup CRUD test database");
}

#[rstest]
fn find_by_id_returns_none_for_missing(crud_context: Result<CrudTestContext, BoxError>) {
    let context = crud_context.expect("failed to create CRUD test context");

    let result = context
        .rt
        .block_on(context.repo.find_by_id(MessageId::new()))
        .expect("failed to load missing message");
    assert!(result.is_none());

    context
        .cleanup()
        .expect("failed to cleanup CRUD test database");
}

#[rstest]
fn find_by_conversation_returns_ordered_messages(
    clock: DefaultClock,
    crud_context: Result<CrudTestContext, BoxError>,
) {
    let context = crud_context.expect("failed to create CRUD test context");

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)
        .expect("failed to insert conversation");

    let msg3 = create_test_message(&clock, conv_id, 3).expect("failed to create message 3");
    let msg1 = create_test_message(&clock, conv_id, 1).expect("failed to create message 1");
    let msg2 = create_test_message(&clock, conv_id, 2).expect("failed to create message 2");

    context
        .rt
        .block_on(context.repo.store(&msg3))
        .expect("failed to store message 3");
    context
        .rt
        .block_on(context.repo.store(&msg1))
        .expect("failed to store message 1");
    context
        .rt
        .block_on(context.repo.store(&msg2))
        .expect("failed to store message 2");

    let messages = context
        .rt
        .block_on(context.repo.find_by_conversation(conv_id))
        .expect("failed to fetch messages");

    assert_eq!(messages.len(), 3);
    let sequence_numbers: Vec<_> = messages
        .iter()
        .map(|message| message.sequence_number().value())
        .collect();
    assert_eq!(sequence_numbers, vec![1, 2, 3]);

    context
        .cleanup()
        .expect("failed to cleanup CRUD test database");
}

#[rstest]
fn exists_returns_correct_status(
    clock: DefaultClock,
    crud_context: Result<CrudTestContext, BoxError>,
) {
    let context = crud_context.expect("failed to create CRUD test context");

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)
        .expect("failed to insert conversation");

    let message = create_test_message(&clock, conv_id, 1).expect("failed to create message");
    let msg_id = message.id();

    let exists_before = context
        .rt
        .block_on(context.repo.exists(msg_id))
        .expect("failed to check message existence");
    assert!(!exists_before);

    context
        .rt
        .block_on(context.repo.store(&message))
        .expect("failed to store message");
    let exists_after = context
        .rt
        .block_on(context.repo.exists(msg_id))
        .expect("failed to check message existence");
    assert!(exists_after);

    context
        .cleanup()
        .expect("failed to cleanup CRUD test database");
}
