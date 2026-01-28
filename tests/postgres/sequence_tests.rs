//! Sequence number management tests for `PostgreSQL` message repository.

use crate::postgres::helpers::{
    BoxError, PostgresCluster, clock, create_test_message, ensure_template, insert_conversation,
    postgres_cluster, setup_repository,
};
use corbusier::message::{domain::ConversationId, ports::repository::MessageRepository};
use mockable::DefaultClock;
use rstest::rstest;

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_one_for_empty(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (_temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    let next = repo.next_sequence_number(conv_id).await?;

    assert_eq!(next.value(), 1);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn next_sequence_number_returns_max_plus_one(
    clock: DefaultClock,
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<(), BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    let (temp_db, repo) = setup_repository(cluster).await?;

    let conv_id = ConversationId::new();
    insert_conversation(cluster, temp_db.name(), conv_id).await?;

    repo.store(&create_test_message(&clock, conv_id, 1)?)
        .await?;
    repo.store(&create_test_message(&clock, conv_id, 2)?)
        .await?;
    repo.store(&create_test_message(&clock, conv_id, 5)?)
        .await?;

    let next = repo.next_sequence_number(conv_id).await?;

    assert_eq!(next.value(), 6);
    Ok(())
}
