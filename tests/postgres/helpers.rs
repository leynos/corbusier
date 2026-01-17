//! Shared test helpers for `PostgreSQL` integration tests.

use corbusier::message::{
    adapters::postgres::PostgresMessageRepository,
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use mockable::DefaultClock;
use pg_embedded_setup_unpriv::TestCluster;
use rstest::fixture;
use tokio::runtime::Runtime;

/// SQL to create the base schema for tests.
pub const CREATE_SCHEMA_SQL: &str =
    include_str!("../../migrations/2025-01-15-000000_create_base_tables/up.sql");

/// SQL to add uniqueness constraints.
pub const ADD_CONSTRAINTS_SQL: &str =
    include_str!("../../migrations/2025-01-15-000001_add_message_uniqueness_constraints/up.sql");

/// SQL to add audit trigger.
pub const ADD_AUDIT_TRIGGER_SQL: &str =
    include_str!("../../migrations/2025-01-16-000000_add_audit_trigger/up.sql");

/// Template database name for pre-migrated schema.
pub const TEMPLATE_DB: &str = "corbusier_test_template";

/// Provides a [`DefaultClock`] for test fixtures.
#[fixture]
pub fn clock() -> DefaultClock {
    DefaultClock
}

/// Creates a tokio runtime for async operations in tests.
#[expect(
    clippy::expect_used,
    reason = "Test helper panics on runtime creation failure"
)]
pub fn test_runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create test runtime")
}

/// Ensures the template database exists with the schema applied.
pub fn ensure_template(
    cluster: &TestCluster,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    cluster
        .ensure_template_exists(TEMPLATE_DB, |db_name| {
            let url = cluster.connection().database_url(db_name);
            let mut conn = PgConnection::establish(&url).map_err(|e| eyre::eyre!("{e}"))?;
            conn.batch_execute(CREATE_SCHEMA_SQL)
                .map_err(|e| eyre::eyre!("SQL error: {e}"))?;
            conn.batch_execute(ADD_CONSTRAINTS_SQL)
                .map_err(|e| eyre::eyre!("SQL error: {e}"))?;
            conn.batch_execute(ADD_AUDIT_TRIGGER_SQL)
                .map_err(|e| eyre::eyre!("SQL error: {e}"))?;
            Ok(())
        })
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(())
}

/// Creates a test database from template and returns a repository.
pub fn setup_repository(
    cluster: &TestCluster,
    db_name: &str,
) -> Result<PostgresMessageRepository, Box<dyn std::error::Error + Send + Sync>> {
    cluster
        .create_database_from_template(db_name, TEMPLATE_DB)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    let url = cluster.connection().database_url(db_name);
    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(PostgresMessageRepository::new(pool))
}

/// Creates a test message with the given conversation and sequence.
#[expect(
    clippy::expect_used,
    reason = "Test helper panics on invalid test message"
)]
pub fn create_test_message(
    clock: &DefaultClock,
    conversation_id: ConversationId,
    sequence: u64,
) -> Message {
    Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Test message content"))],
        SequenceNumber::new(sequence),
        clock,
    )
    .expect("valid test message")
}

/// Cleans up a test database.
#[expect(
    clippy::print_stderr,
    reason = "Test cleanup warnings are informational"
)]
pub fn cleanup_database(cluster: &TestCluster, db_name: &str) {
    if let Err(e) = cluster.drop_database(db_name) {
        eprintln!("Warning: failed to drop test database {db_name}: {e}");
    }
}

/// Guard that ensures test database cleanup runs even if test panics.
pub struct CleanupGuard<'a> {
    cluster: &'a TestCluster,
    db_name: String,
}

impl<'a> CleanupGuard<'a> {
    pub const fn new(cluster: &'a TestCluster, db_name: String) -> Self {
        Self { cluster, db_name }
    }
}

impl Drop for CleanupGuard<'_> {
    fn drop(&mut self) {
        cleanup_database(self.cluster, &self.db_name);
    }
}

/// Helper struct for querying role from database.
#[derive(diesel::QueryableByName)]
pub struct RoleResult {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub role: String,
}

/// Inserts a conversation record to satisfy the foreign key constraint.
#[expect(clippy::expect_used, reason = "Test helper panics on insert failure")]
pub fn insert_conversation(cluster: &TestCluster, db_name: &str, conv_id: ConversationId) {
    let url = cluster.connection().database_url(db_name);
    let mut conn = PgConnection::establish(&url).expect("connection");

    diesel::sql_query(concat!(
        "INSERT INTO conversations (id, context, state, created_at, updated_at) ",
        "VALUES ($1, '{}', 'active', NOW(), NOW())",
    ))
    .bind::<diesel::sql_types::Uuid, _>(conv_id.into_inner())
    .execute(&mut conn)
    .expect("insert conversation");
}

/// Row from the `audit_logs` table for verification.
#[expect(
    dead_code,
    reason = "Fields are populated by Diesel but not all read in tests"
)]
#[derive(diesel::QueryableByName, Debug)]
pub struct AuditLogRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: uuid::Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub table_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub operation: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    pub row_id: Option<uuid::Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    pub correlation_id: Option<uuid::Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    pub causation_id: Option<uuid::Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    pub user_id: Option<uuid::Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    pub session_id: Option<uuid::Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub application_name: Option<String>,
}

/// Fetches the audit log entry for a specific message row ID.
///
/// Returns `Ok(Some(row))` if found, `Ok(None)` if not found, or `Err` on query failure.
#[expect(
    clippy::expect_used,
    reason = "Test helper panics on connection failure"
)]
pub fn fetch_audit_log_for_message(
    cluster: &TestCluster,
    db_name: &str,
    message_id: uuid::Uuid,
) -> Result<Option<AuditLogRow>, diesel::result::Error> {
    let url = cluster.connection().database_url(db_name);
    let mut conn = PgConnection::establish(&url).expect("connection");

    match diesel::sql_query(concat!(
        "SELECT id, table_name, operation, row_id, correlation_id, causation_id, ",
        "user_id, session_id, application_name ",
        "FROM audit_logs WHERE row_id = $1 ORDER BY occurred_at DESC LIMIT 1",
    ))
    .bind::<diesel::sql_types::Uuid, _>(message_id)
    .get_result::<AuditLogRow>(&mut conn)
    {
        Ok(row) => Ok(Some(row)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(e),
    }
}
