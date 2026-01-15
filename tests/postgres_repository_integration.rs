//! Integration tests for [`PostgresMessageRepository`] using embedded `PostgreSQL`.
//!
//! These tests exercise the `PostgreSQL` repository implementation against a real
//! database instance, verifying CRUD operations, uniqueness constraints, and
//! error handling.
//!
//! Uses `pg-embed-setup-unpriv` for embedded `PostgreSQL` lifecycle management.

#![expect(
    clippy::expect_used,
    reason = "Test code uses expect for assertion clarity"
)]
#![expect(
    clippy::indexing_slicing,
    reason = "Test code uses indexing after length checks"
)]
#![expect(
    clippy::print_stderr,
    reason = "Test cleanup warnings are informational"
)]

use corbusier::message::{
    adapters::postgres::PostgresMessageRepository,
    domain::{ContentPart, ConversationId, Message, MessageId, Role, SequenceNumber, TextPart},
    error::RepositoryError,
    ports::repository::MessageRepository,
};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use mockable::DefaultClock;
use pg_embedded_setup_unpriv::{TestCluster, test_support::shared_test_cluster};
use rstest::rstest;

/// SQL to create the base schema for tests.
const CREATE_SCHEMA_SQL: &str =
    include_str!("../migrations/2025-01-15-000000_create_base_tables/up.sql");

/// SQL to add uniqueness constraints.
const ADD_CONSTRAINTS_SQL: &str =
    include_str!("../migrations/2025-01-15-000001_add_message_uniqueness_constraints/up.sql");

/// Template database name for pre-migrated schema.
const TEMPLATE_DB: &str = "corbusier_test_template";

/// Ensures the template database exists with the schema applied.
fn ensure_template(cluster: &TestCluster) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    cluster
        .ensure_template_exists(TEMPLATE_DB, |db_name| {
            let url = cluster.connection().database_url(db_name);
            let mut conn = PgConnection::establish(&url).map_err(|e| eyre::eyre!("{e}"))?;
            diesel::sql_query(CREATE_SCHEMA_SQL)
                .execute(&mut conn)
                .map_err(|e| eyre::eyre!("{e}"))?;
            diesel::sql_query(ADD_CONSTRAINTS_SQL)
                .execute(&mut conn)
                .map_err(|e| eyre::eyre!("{e}"))?;
            Ok(())
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(())
}

/// Creates a test database from template and returns a repository.
fn setup_repository(
    cluster: &TestCluster,
    db_name: &str,
) -> Result<PostgresMessageRepository, Box<dyn std::error::Error + Send + Sync>> {
    cluster
        .create_database_from_template(db_name, TEMPLATE_DB)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    let url = cluster.connection().database_url(db_name);
    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool = Pool::builder()
        .max_size(2)
        .build(manager)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(PostgresMessageRepository::new(pool))
}

/// Creates a test message with the given conversation and sequence.
fn create_test_message(conversation_id: ConversationId, sequence: u64) -> Message {
    let clock = DefaultClock;
    Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Test message content"))],
        SequenceNumber::new(sequence),
        &clock,
    )
    .expect("valid test message")
}

/// Cleans up a test database.
fn cleanup_database(cluster: &TestCluster, db_name: &str) {
    if let Err(e) = cluster.drop_database(db_name) {
        eprintln!("Warning: failed to drop test database {db_name}: {e}");
    }
}

// ============================================================================
// Basic CRUD Operations
// ============================================================================

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn store_and_retrieve_message(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_store_retrieve_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(conv_id, 1);
    let msg_id = message.id();

    // Store
    repo.store(&message).await.expect("store should succeed");

    // Retrieve by ID
    let retrieved = repo
        .find_by_id(msg_id)
        .await
        .expect("find_by_id should succeed")
        .expect("message should exist");

    assert_eq!(retrieved.id(), msg_id);
    assert_eq!(retrieved.conversation_id(), conv_id);
    assert_eq!(retrieved.role(), Role::User);
    assert_eq!(retrieved.sequence_number().value(), 1);

    cleanup_database(shared_test_cluster, &db_name);
}

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn find_by_id_returns_none_for_missing(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_find_none_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let result = repo.find_by_id(MessageId::new()).await.expect("query ok");
    assert!(result.is_none());

    cleanup_database(shared_test_cluster, &db_name);
}

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn find_by_conversation_returns_ordered_messages(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_find_conv_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    // Store messages out of order
    let msg3 = create_test_message(conv_id, 3);
    let msg1 = create_test_message(conv_id, 1);
    let msg2 = create_test_message(conv_id, 2);

    repo.store(&msg3).await.expect("store msg3");
    repo.store(&msg1).await.expect("store msg1");
    repo.store(&msg2).await.expect("store msg2");

    // Retrieve should return in sequence order
    let messages = repo
        .find_by_conversation(conv_id)
        .await
        .expect("find_by_conversation");

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].sequence_number().value(), 1);
    assert_eq!(messages[1].sequence_number().value(), 2);
    assert_eq!(messages[2].sequence_number().value(), 3);

    cleanup_database(shared_test_cluster, &db_name);
}

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn exists_returns_correct_status(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_exists_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(conv_id, 1);
    let msg_id = message.id();

    // Before store
    assert!(!repo.exists(msg_id).await.expect("exists check"));

    // After store
    repo.store(&message).await.expect("store");
    assert!(repo.exists(msg_id).await.expect("exists check"));

    cleanup_database(shared_test_cluster, &db_name);
}

// ============================================================================
// Sequence Number Management
// ============================================================================

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn next_sequence_number_returns_one_for_empty(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_next_seq_empty_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    let next = repo
        .next_sequence_number(conv_id)
        .await
        .expect("next_sequence_number");

    assert_eq!(next.value(), 1);

    cleanup_database(shared_test_cluster, &db_name);
}

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn next_sequence_number_returns_max_plus_one(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_next_seq_incr_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    // Store messages with sequence 1, 2, 5 (gap)
    repo.store(&create_test_message(conv_id, 1))
        .await
        .expect("store 1");
    repo.store(&create_test_message(conv_id, 2))
        .await
        .expect("store 2");
    repo.store(&create_test_message(conv_id, 5))
        .await
        .expect("store 5");

    let next = repo
        .next_sequence_number(conv_id)
        .await
        .expect("next_sequence_number");

    assert_eq!(next.value(), 6); // max(5) + 1

    cleanup_database(shared_test_cluster, &db_name);
}

// ============================================================================
// Uniqueness Constraints
// ============================================================================

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn store_rejects_duplicate_message_id(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_dup_msg_id_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(conv_id, 1);
    let msg_id = message.id();

    // First store succeeds
    repo.store(&message).await.expect("first store");

    // Create another message with the same ID but different sequence
    let clock = DefaultClock;
    let duplicate = Message::builder(conv_id, Role::User, SequenceNumber::new(2))
        .with_id(msg_id)
        .with_content(ContentPart::Text(TextPart::new("Different content")))
        .build(&clock)
        .expect("duplicate message");

    // Second store should fail with DuplicateMessage
    let result = repo.store(&duplicate).await;
    assert!(
        matches!(result, Err(RepositoryError::DuplicateMessage(id)) if id == msg_id),
        "Expected DuplicateMessage error, got: {result:?}"
    );

    cleanup_database(shared_test_cluster, &db_name);
}

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn store_rejects_duplicate_sequence_in_conversation(
    shared_test_cluster: &'static TestCluster,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_dup_seq_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    // First message with sequence 1
    let msg1 = create_test_message(conv_id, 1);
    repo.store(&msg1).await.expect("first store");

    // Second message with same sequence 1 but different ID
    let msg2 = create_test_message(conv_id, 1);

    // Should fail with DuplicateSequence
    let result = repo.store(&msg2).await;
    assert!(
        matches!(
            result,
            Err(RepositoryError::DuplicateSequence {
                conversation_id: c,
                sequence: s
            }) if c == conv_id && s.value() == 1
        ),
        "Expected DuplicateSequence error, got: {result:?}"
    );

    cleanup_database(shared_test_cluster, &db_name);
}

#[rstest]
#[tokio::test]
#[ignore = "Requires embedded PostgreSQL infrastructure (set PG_EMBEDDED_WORKER)"]
async fn store_allows_same_sequence_in_different_conversations(
    shared_test_cluster: &'static TestCluster,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_diff_conv_seq_{}", uuid::Uuid::new_v4());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv1 = ConversationId::new();
    let conv2 = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv1);
    insert_conversation(shared_test_cluster, &db_name, conv2);

    // Both messages have sequence 1 but different conversations
    let msg1 = create_test_message(conv1, 1);
    let msg2 = create_test_message(conv2, 1);

    // Both should succeed
    repo.store(&msg1).await.expect("store in conv1");
    repo.store(&msg2).await.expect("store in conv2");

    // Verify both exist
    assert!(repo.exists(msg1.id()).await.expect("exists check"));
    assert!(repo.exists(msg2.id()).await.expect("exists check"));

    cleanup_database(shared_test_cluster, &db_name);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Inserts a conversation record to satisfy the foreign key constraint.
fn insert_conversation(cluster: &TestCluster, db_name: &str, conv_id: ConversationId) {
    let url = cluster.connection().database_url(db_name);
    let mut conn = PgConnection::establish(&url).expect("connection");

    diesel::sql_query(
        "INSERT INTO conversations (id, context, state, created_at, updated_at) \
         VALUES ($1, '{}', 'active', NOW(), NOW())",
    )
    .bind::<diesel::sql_types::Uuid, _>(conv_id.into_inner())
    .execute(&mut conn)
    .expect("insert conversation");
}
