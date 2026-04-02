//! Sequence number management tests for `PostgreSQL` message repository.

use crate::postgres::helpers::{
    BoxError, PostgresCluster, clock, create_test_message, ensure_template, insert_conversation,
    postgres_cluster, setup_repository, test_request_context,
};
use corbusier::context::RequestContext;
use corbusier::message::{domain::ConversationId, ports::repository::MessageRepository};
use mockable::DefaultClock;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_one_for_empty(
    postgres_cluster: Result<PostgresCluster, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let ctx = test_request_context;
    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id, &ctx).await?;
    let next = repo.next_sequence_number(&ctx, conv_id).await?;

    assert_eq!(next.value(), 1);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_conversation_not_found_for_missing_conversation(
    postgres_cluster: Result<PostgresCluster, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (_temp_db, repo) = setup_repository(cluster).await?;

    let ctx = test_request_context;
    let conv_id = ConversationId::new();
    let result = repo.next_sequence_number(&ctx, conv_id).await;

    assert!(matches!(
        result,
        Err(corbusier::message::error::RepositoryError::ConversationNotFound(id))
            if id == conv_id
    ));
    Ok(())
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_max_plus_one(
    clock: DefaultClock,
    postgres_cluster: Result<PostgresCluster, BoxError>,
    test_request_context: RequestContext,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let ctx = test_request_context;
    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id, &ctx).await?;

    repo.store(&ctx, &create_test_message(&clock, conv_id, 1)?)
        .await?;
    repo.store(&ctx, &create_test_message(&clock, conv_id, 2)?)
        .await?;
    repo.store(&ctx, &create_test_message(&clock, conv_id, 5)?)
        .await?;

    let next = repo.next_sequence_number(&ctx, conv_id).await?;

    assert_eq!(next.value(), 6);
    Ok(())
}
