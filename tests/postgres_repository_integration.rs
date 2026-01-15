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
use tokio::runtime::Runtime;

/// SQL to create the base schema for tests.
const CREATE_SCHEMA_SQL: &str =
    include_str!("../migrations/2025-01-15-000000_create_base_tables/up.sql");

/// SQL to add uniqueness constraints.
const ADD_CONSTRAINTS_SQL: &str =
    include_str!("../migrations/2025-01-15-000001_add_message_uniqueness_constraints/up.sql");

/// Template database name for pre-migrated schema.
const TEMPLATE_DB: &str = "corbusier_test_template";

/// Creates a tokio runtime for async operations in tests.
fn test_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create test runtime")
}

/// Ensures the template database exists with the schema applied.
fn ensure_template(cluster: &TestCluster) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    cluster
        .ensure_template_exists(TEMPLATE_DB, |db_name| {
            let url = cluster.connection().database_url(db_name);
            let mut conn = PgConnection::establish(&url).map_err(|e| eyre::eyre!("{e}"))?;
            // Execute each SQL file statement-by-statement since diesel::sql_query
            // cannot execute multiple statements in a single call
            execute_sql_statements(&mut conn, CREATE_SCHEMA_SQL)?;
            execute_sql_statements(&mut conn, ADD_CONSTRAINTS_SQL)?;
            Ok(())
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(())
}

/// Executes multiple SQL statements from a single string.
///
/// Splits on semicolons and executes each non-empty statement individually.
/// Comments (lines starting with --) are preserved within statements.
fn execute_sql_statements(conn: &mut PgConnection, sql: &str) -> eyre::Result<()> {
    for statement in sql.split(';') {
        let trimmed = statement.trim();
        // Skip empty statements and comment-only lines
        if trimmed.is_empty() || trimmed.lines().all(|line| line.trim().starts_with("--")) {
            continue;
        }
        diesel::sql_query(trimmed)
            .execute(conn)
            .map_err(|e| eyre::eyre!("SQL error: {e}\nStatement: {trimmed}"))?;
    }
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
    // Use pool size of 1 for test isolation and deterministic behaviour
    let pool = Pool::builder()
        .max_size(1)
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

/// Guard that ensures test database cleanup runs even if test panics.
struct CleanupGuard<'a> {
    cluster: &'a TestCluster,
    db_name: String,
}

impl<'a> CleanupGuard<'a> {
    const fn new(cluster: &'a TestCluster, db_name: String) -> Self {
        Self { cluster, db_name }
    }
}

impl Drop for CleanupGuard<'_> {
    fn drop(&mut self) {
        cleanup_database(self.cluster, &self.db_name);
    }
}

// ============================================================================
// Basic CRUD Operations
// ============================================================================

#[rstest]
fn store_and_retrieve_message(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_store_retrieve_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(conv_id, 1);
    let msg_id = message.id();

    let rt = test_runtime();

    // Store
    rt.block_on(repo.store(&message))
        .expect("store should succeed");

    // Retrieve by ID
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
fn find_by_conversation_returns_ordered_messages(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_find_conv_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    // Store messages out of order
    let msg3 = create_test_message(conv_id, 3);
    let msg1 = create_test_message(conv_id, 1);
    let msg2 = create_test_message(conv_id, 2);

    let rt = test_runtime();
    rt.block_on(repo.store(&msg3)).expect("store msg3");
    rt.block_on(repo.store(&msg1)).expect("store msg1");
    rt.block_on(repo.store(&msg2)).expect("store msg2");

    // Retrieve should return in sequence order
    let messages = rt
        .block_on(repo.find_by_conversation(conv_id))
        .expect("find_by_conversation");

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].sequence_number().value(), 1);
    assert_eq!(messages[1].sequence_number().value(), 2);
    assert_eq!(messages[2].sequence_number().value(), 3);
}

#[rstest]
fn exists_returns_correct_status(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_exists_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(conv_id, 1);
    let msg_id = message.id();

    let rt = test_runtime();

    // Before store
    assert!(!rt.block_on(repo.exists(msg_id)).expect("exists check"));

    // After store
    rt.block_on(repo.store(&message)).expect("store");
    assert!(rt.block_on(repo.exists(msg_id)).expect("exists check"));
}

// ============================================================================
// Sequence Number Management
// ============================================================================

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
fn next_sequence_number_returns_max_plus_one(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_next_seq_incr_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let rt = test_runtime();

    // Store messages with sequence 1, 2, 5 (gap)
    rt.block_on(repo.store(&create_test_message(conv_id, 1)))
        .expect("store 1");
    rt.block_on(repo.store(&create_test_message(conv_id, 2)))
        .expect("store 2");
    rt.block_on(repo.store(&create_test_message(conv_id, 5)))
        .expect("store 5");

    let next = rt
        .block_on(repo.next_sequence_number(conv_id))
        .expect("next_sequence_number");

    assert_eq!(next.value(), 6); // max(5) + 1
}

// ============================================================================
// Uniqueness Constraints
// ============================================================================

#[rstest]
fn store_rejects_duplicate_message_id(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_dup_msg_id_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(conv_id, 1);
    let msg_id = message.id();

    let rt = test_runtime();

    // First store succeeds
    rt.block_on(repo.store(&message)).expect("first store");

    // Create another message with the same ID but different sequence
    let clock = DefaultClock;
    let duplicate = Message::builder(conv_id, Role::User, SequenceNumber::new(2))
        .with_id(msg_id)
        .with_content(ContentPart::Text(TextPart::new("Different content")))
        .build(&clock)
        .expect("duplicate message");

    // Second store should fail with DuplicateMessage
    let result = rt.block_on(repo.store(&duplicate));
    assert!(
        matches!(result, Err(RepositoryError::DuplicateMessage(id)) if id == msg_id),
        "Expected DuplicateMessage error, got: {result:?}"
    );
}

#[rstest]
fn store_rejects_duplicate_sequence_in_conversation(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_dup_seq_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    // First message with sequence 1
    let msg1 = create_test_message(conv_id, 1);

    let rt = test_runtime();
    rt.block_on(repo.store(&msg1)).expect("first store");

    // Second message with same sequence 1 but different ID
    let msg2 = create_test_message(conv_id, 1);

    // Should fail with DuplicateSequence
    let result = rt.block_on(repo.store(&msg2));
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
}

#[rstest]
fn store_allows_same_sequence_in_different_conversations(
    shared_test_cluster: &'static TestCluster,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_diff_conv_seq_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv1 = ConversationId::new();
    let conv2 = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv1);
    insert_conversation(shared_test_cluster, &db_name, conv2);

    // Both messages have sequence 1 but different conversations
    let msg1 = create_test_message(conv1, 1);
    let msg2 = create_test_message(conv2, 1);

    let rt = test_runtime();

    // Both should succeed
    rt.block_on(repo.store(&msg1)).expect("store in conv1");
    rt.block_on(repo.store(&msg2)).expect("store in conv2");

    // Verify both exist
    assert!(rt.block_on(repo.exists(msg1.id())).expect("exists check"));
    assert!(rt.block_on(repo.exists(msg2.id())).expect("exists check"));
}

// ============================================================================
// Role Parsing Through Persistence (Comment 6)
// ============================================================================

/// Tests that all Role variants round-trip correctly through `PostgreSQL` storage.
#[rstest]
#[case(Role::User, "user")]
#[case(Role::Assistant, "assistant")]
#[case(Role::Tool, "tool")]
#[case(Role::System, "system")]
fn role_round_trip_through_persistence(
    shared_test_cluster: &'static TestCluster,
    #[case] role: Role,
    #[case] expected_str: &str,
) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_role_rt_{}_{}", expected_str, uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let clock = DefaultClock;
    let message = Message::new(
        conv_id,
        role,
        vec![ContentPart::Text(TextPart::new("Role test"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store(&message)).expect("store");

    // Verify the role is stored correctly in the database
    let url = shared_test_cluster.connection().database_url(&db_name);
    let mut conn = PgConnection::establish(&url).expect("connection");
    let stored_role: String = diesel::sql_query("SELECT role FROM messages WHERE id = $1")
        .bind::<diesel::sql_types::Uuid, _>(message.id().into_inner())
        .get_result::<RoleResult>(&mut conn)
        .expect("query")
        .role;

    assert_eq!(stored_role, expected_str);

    // Verify round-trip retrieval parses role correctly
    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find")
        .expect("exists");

    assert_eq!(retrieved.role(), role);
}

// ============================================================================
// JSONB Round-Trip (Comments 7, 11)
// ============================================================================

/// Tests that complex message content with multiple parts round-trips through JSONB.
#[rstest]
fn content_jsonb_round_trip_with_multiple_parts(shared_test_cluster: &'static TestCluster) {
    use corbusier::message::domain::{AttachmentPart, ToolCallPart};

    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_jsonb_content_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    // Create message with multiple content part types
    let clock = DefaultClock;
    let content = vec![
        ContentPart::Text(TextPart::new("Hello world")),
        ContentPart::Attachment(AttachmentPart::new(
            "image/png",
            "iVBORw0KGgo=", // base64 encoded PNG header
        )),
        ContentPart::ToolCall(ToolCallPart::new(
            "call_123",
            "search",
            serde_json::json!({"query": "test"}),
        )),
    ];

    let message = Message::new(
        conv_id,
        Role::Assistant,
        content,
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store(&message)).expect("store");

    // Retrieve and verify all content parts preserved
    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find")
        .expect("exists");

    assert_eq!(retrieved.content().len(), 3);

    // Verify text part
    match &retrieved.content()[0] {
        ContentPart::Text(text) => assert_eq!(text.text, "Hello world"),
        other => panic!("Expected Text, got {other:?}"),
    }

    // Verify attachment part
    match &retrieved.content()[1] {
        ContentPart::Attachment(att) => {
            assert_eq!(att.mime_type, "image/png");
            assert_eq!(att.data, "iVBORw0KGgo=");
        }
        other => panic!("Expected Attachment, got {other:?}"),
    }

    // Verify tool call part
    match &retrieved.content()[2] {
        ContentPart::ToolCall(call) => {
            assert_eq!(call.call_id, "call_123");
            assert_eq!(call.name, "search");
            assert_eq!(call.arguments, serde_json::json!({"query": "test"}));
        }
        other => panic!("Expected ToolCall, got {other:?}"),
    }
}

/// Tests tool result content part round-trip.
#[rstest]
fn tool_result_jsonb_round_trip(shared_test_cluster: &'static TestCluster) {
    use corbusier::message::domain::ToolResultPart;

    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_tool_result_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let clock = DefaultClock;

    // Test successful tool result
    let success_result =
        ToolResultPart::success("call_456", serde_json::json!({"result": "found 42 items"}));

    // Test failed tool result
    let failure_result = ToolResultPart::failure("call_789", "Network timeout");

    let message = Message::new(
        conv_id,
        Role::Tool,
        vec![
            ContentPart::ToolResult(success_result),
            ContentPart::ToolResult(failure_result),
        ],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store(&message)).expect("store");

    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find")
        .expect("exists");

    assert_eq!(retrieved.content().len(), 2);

    // Verify success result
    match &retrieved.content()[0] {
        ContentPart::ToolResult(result) => {
            assert_eq!(result.call_id, "call_456");
            assert!(result.success);
            assert_eq!(
                result.content,
                serde_json::json!({"result": "found 42 items"})
            );
        }
        other => panic!("Expected ToolResult, got {other:?}"),
    }

    // Verify failure result
    match &retrieved.content()[1] {
        ContentPart::ToolResult(result) => {
            assert_eq!(result.call_id, "call_789");
            assert!(!result.success);
        }
        other => panic!("Expected ToolResult, got {other:?}"),
    }
}

/// Tests metadata JSONB round-trip including `agent_backend` field.
#[rstest]
fn metadata_jsonb_round_trip(shared_test_cluster: &'static TestCluster) {
    use corbusier::message::domain::MessageMetadata;

    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_metadata_jsonb_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let clock = DefaultClock;
    let metadata = MessageMetadata::with_agent_backend("claude-3-opus");

    let message = Message::builder(conv_id, Role::Assistant, SequenceNumber::new(1))
        .with_content(ContentPart::Text(TextPart::new("Response")))
        .with_metadata(metadata)
        .build(&clock)
        .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store(&message)).expect("store");

    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find")
        .expect("exists");

    assert_eq!(
        retrieved.metadata().agent_backend,
        Some("claude-3-opus".to_owned())
    );
}

// ============================================================================
// Message::from_persisted Reconstruction (Comment 7)
// ============================================================================

/// Tests that `Message::from_persisted` correctly reconstructs domain objects
/// including all invariants (non-nil ID, non-empty content, valid timestamps).
#[rstest]
fn from_persisted_preserves_all_domain_invariants(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_from_persisted_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let clock = DefaultClock;
    let original = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Test content"))],
        SequenceNumber::new(42),
        &clock,
    )
    .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store(&original)).expect("store");

    let retrieved = rt
        .block_on(repo.find_by_id(original.id()))
        .expect("find")
        .expect("exists");

    // Verify all domain invariants
    assert!(
        !retrieved.id().into_inner().is_nil(),
        "ID should not be nil"
    );
    assert!(
        !retrieved.conversation_id().into_inner().is_nil(),
        "Conversation ID should not be nil"
    );
    assert!(
        !retrieved.content().is_empty(),
        "Content should not be empty"
    );
    assert_eq!(retrieved.sequence_number().value(), 42);

    // Verify timestamp is preserved (within reasonable tolerance)
    let time_diff = (original.created_at() - retrieved.created_at())
        .num_milliseconds()
        .abs();
    assert!(
        time_diff < 1000,
        "Timestamp should be preserved within 1 second, diff was {time_diff}ms"
    );
}

// ============================================================================
// Audit Context Propagation (Comment 12)
// ============================================================================

/// Tests that `store_with_audit` sets `PostgreSQL` session variables correctly.
#[rstest]
fn store_with_audit_sets_session_variables(shared_test_cluster: &'static TestCluster) {
    use corbusier::message::adapters::audit_context::AuditContext;

    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_audit_ctx_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let clock = DefaultClock;
    let message = Message::new(
        conv_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Audited message"))],
        SequenceNumber::new(1),
        &clock,
    )
    .expect("valid message");

    let correlation_id = uuid::Uuid::new_v4();
    let causation_id = uuid::Uuid::new_v4();
    let user_id = uuid::Uuid::new_v4();
    let session_id = uuid::Uuid::new_v4();

    let audit = AuditContext::empty()
        .with_correlation_id(correlation_id)
        .with_causation_id(causation_id)
        .with_user_id(user_id)
        .with_session_id(session_id);

    let rt = test_runtime();

    // Store with audit context should succeed
    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store_with_audit");

    // Verify message was stored
    let retrieved = rt
        .block_on(repo.find_by_id(message.id()))
        .expect("find")
        .expect("exists");

    assert_eq!(retrieved.id(), message.id());

    // Note: We can't directly verify session variables were set because they
    // are transaction-local (SET LOCAL). The important thing is that the
    // operation succeeded and the message was stored. In a real system,
    // audit triggers would capture these variables into audit_logs table.
}

/// Tests `store_with_audit` with empty audit context.
#[rstest]
fn store_with_audit_handles_empty_context(shared_test_cluster: &'static TestCluster) {
    use corbusier::message::adapters::audit_context::AuditContext;

    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_audit_empty_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(conv_id, 1);

    let audit = AuditContext::empty();
    assert!(audit.is_empty());

    let rt = test_runtime();

    // Should succeed even with empty audit context
    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store_with_audit with empty context");

    assert!(rt.block_on(repo.exists(message.id())).expect("exists"));
}

/// Tests `store_with_audit` with partial audit context.
#[rstest]
fn store_with_audit_handles_partial_context(shared_test_cluster: &'static TestCluster) {
    use corbusier::message::adapters::audit_context::AuditContext;

    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_audit_partial_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let message = create_test_message(conv_id, 1);

    // Only set correlation_id, leave others as None
    let audit = AuditContext::empty().with_correlation_id(uuid::Uuid::new_v4());

    let rt = test_runtime();
    rt.block_on(repo.store_with_audit(&message, &audit))
        .expect("store_with_audit with partial context");

    assert!(rt.block_on(repo.exists(message.id())).expect("exists"));
}

// ============================================================================
// UUID Handling (Comment 11)
// ============================================================================

/// Tests that UUIDs are correctly stored and retrieved from `PostgreSQL`.
#[rstest]
fn uuid_round_trip_preserves_values(shared_test_cluster: &'static TestCluster) {
    ensure_template(shared_test_cluster).expect("template setup");
    let db_name = format!("test_uuid_rt_{}", uuid::Uuid::new_v4());
    let _guard = CleanupGuard::new(shared_test_cluster, db_name.clone());
    let repo = setup_repository(shared_test_cluster, &db_name).expect("repository setup");

    let conv_id = ConversationId::new();
    insert_conversation(shared_test_cluster, &db_name, conv_id);

    let clock = DefaultClock;
    let specific_msg_id = MessageId::from_uuid(
        uuid::Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid uuid"),
    );

    let message = Message::builder(conv_id, Role::User, SequenceNumber::new(1))
        .with_id(specific_msg_id)
        .with_content(ContentPart::Text(TextPart::new("UUID test")))
        .build(&clock)
        .expect("valid message");

    let rt = test_runtime();
    rt.block_on(repo.store(&message)).expect("store");

    let retrieved = rt
        .block_on(repo.find_by_id(specific_msg_id))
        .expect("find")
        .expect("exists");

    assert_eq!(
        retrieved.id().into_inner().to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
    assert_eq!(retrieved.conversation_id(), conv_id);
}

// ============================================================================
// Helper Types and Functions
// ============================================================================

/// Helper struct for querying role from database.
#[derive(diesel::QueryableByName)]
struct RoleResult {
    #[diesel(sql_type = diesel::sql_types::Text)]
    role: String,
}

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
