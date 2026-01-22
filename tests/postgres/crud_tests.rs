//! Basic CRUD operation tests for `PostgreSQL` message repository.

use crate::postgres::cluster::{BoxError, TemporaryDatabase};
use crate::postgres::helpers::{
    PostgresCluster, clock, create_test_message, ensure_template, insert_conversation,
    postgres_cluster, setup_repository,
};
use corbusier::message::adapters::postgres::PostgresMessageRepository;
use corbusier::message::{
    domain::{ConversationId, MessageId, Role},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

struct CrudTestContext {
    cluster: PostgresCluster,
    temp_db: TemporaryDatabase,
    repo: PostgresMessageRepository,
}

/// Creates a CRUD test context with database and repository.
///
/// Returns `Result` to allow `?` error propagation. Tests should return
/// `Result` and use `?` to consume the fixture.
#[fixture]
async fn crud_context(postgres_cluster: PostgresCluster) -> Result<CrudTestContext, BoxError> {
    let cluster = postgres_cluster;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;
    Ok(CrudTestContext {
        cluster,
        temp_db,
        repo,
    })
}

#[rstest]
#[tokio::test]
async fn store_and_retrieve_message(
    clock: DefaultClock,
    #[future] crud_context: Result<CrudTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = crud_context.await?;

    let conv_id = ConversationId::new();
    insert_conversation(ctx.cluster, ctx.temp_db.name(), conv_id).await?;

    let message = create_test_message(&clock, conv_id, 1)?;
    let msg_id = message.id();

    ctx.repo.store(&message).await?;

    let retrieved = ctx
        .repo
        .find_by_id(msg_id)
        .await?
        .expect("message should exist");

    assert_eq!(retrieved.id(), msg_id);
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::User);
    assert_eq!(retrieved.sequence_number().value(), 1);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn find_by_id_returns_none_for_missing(
    #[future] crud_context: Result<CrudTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = crud_context.await?;

    let result = ctx.repo.find_by_id(MessageId::new()).await?;
    assert!(result.is_none());
    Ok(())
}

#[rstest]
#[tokio::test]
async fn find_by_conversation_returns_ordered_messages(
    clock: DefaultClock,
    #[future] crud_context: Result<CrudTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = crud_context.await?;

    let conv_id = ConversationId::new();
    insert_conversation(ctx.cluster, ctx.temp_db.name(), conv_id).await?;

    let msg3 = create_test_message(&clock, conv_id, 3)?;
    let msg1 = create_test_message(&clock, conv_id, 1)?;
    let msg2 = create_test_message(&clock, conv_id, 2)?;

    ctx.repo.store(&msg3).await?;
    ctx.repo.store(&msg1).await?;
    ctx.repo.store(&msg2).await?;

    let messages = ctx.repo.find_by_conversation(conv_id).await?;

    assert_eq!(messages.len(), 3);
    let sequence_numbers: Vec<_> = messages
        .iter()
        .map(|message| message.sequence_number().value())
        .collect();
    assert_eq!(sequence_numbers, vec![1, 2, 3]);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn exists_returns_correct_status(
    clock: DefaultClock,
    #[future] crud_context: Result<CrudTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = crud_context.await?;

    let conv_id = ConversationId::new();
    insert_conversation(ctx.cluster, ctx.temp_db.name(), conv_id).await?;

    let message = create_test_message(&clock, conv_id, 1)?;
    let msg_id = message.id();

    let exists_before = ctx.repo.exists(msg_id).await?;
    assert!(!exists_before);

    ctx.repo.store(&message).await?;
    let exists_after = ctx.repo.exists(msg_id).await?;
    assert!(exists_after);
    Ok(())
}
