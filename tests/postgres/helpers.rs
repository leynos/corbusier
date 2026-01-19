//! Shared test helpers for `PostgreSQL` integration tests.

use super::cluster::{BoxError, ManagedCluster};
pub use super::cluster::{PostgresCluster, postgres_cluster};
use corbusier::message::{
    adapters::audit_context::AuditContext,
    adapters::postgres::PostgresMessageRepository,
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart},
};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use mockable::DefaultClock;
use rstest::fixture;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// SQL to create the base schema for tests.
pub const CREATE_SCHEMA_SQL: &str =
    include_str!("../../migrations/2026-01-15-000000_create_base_tables/up.sql");

/// SQL to add uniqueness constraints.
pub const ADD_CONSTRAINTS_SQL: &str =
    include_str!("../../migrations/2026-01-15-000001_add_message_uniqueness_constraints/up.sql");

/// SQL to add audit trigger.
pub const ADD_AUDIT_TRIGGER_SQL: &str =
    include_str!("../../migrations/2026-01-16-000000_add_audit_trigger/up.sql");

/// Template database name for pre-migrated schema.
pub const TEMPLATE_DB: &str = "corbusier_test_template";

/// Provides a [`DefaultClock`] for test fixtures.
#[fixture]
pub fn clock() -> DefaultClock {
    DefaultClock
}

/// Creates a tokio runtime for async operations in tests.
///
/// # Errors
///
/// Returns an error if the runtime cannot be created.
pub fn test_runtime() -> Result<Runtime, BoxError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Box::new(e) as BoxError)
}

/// Ensures the template database exists with the schema applied.
pub fn ensure_template(cluster: &ManagedCluster) -> Result<(), BoxError> {
    cluster.ensure_template_exists(TEMPLATE_DB, |db_name| {
        let url = cluster.connection().database_url(db_name);
        let mut conn = PgConnection::establish(&url).map_err(|err| Box::new(err) as BoxError)?;
        conn.batch_execute(CREATE_SCHEMA_SQL)
            .map_err(|err| Box::new(err) as BoxError)?;
        conn.batch_execute(ADD_CONSTRAINTS_SQL)
            .map_err(|err| Box::new(err) as BoxError)?;
        conn.batch_execute(ADD_AUDIT_TRIGGER_SQL)
            .map_err(|err| Box::new(err) as BoxError)?;
        Ok(())
    })?;
    Ok(())
}

/// Creates a test database from template and returns a repository.
pub fn setup_repository(
    cluster: &ManagedCluster,
    db_name: &str,
) -> Result<PostgresMessageRepository, BoxError> {
    cluster.create_database_from_template(db_name, TEMPLATE_DB)?;
    let url = cluster.connection().database_url(db_name);
    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Box::new(e) as BoxError)?;
    Ok(PostgresMessageRepository::new(pool))
}

/// Creates a test message with the given conversation and sequence.
///
/// # Errors
///
/// Returns an error if message creation fails.
pub fn create_test_message(
    clock: &DefaultClock,
    conversation_id: ConversationId,
    sequence: u64,
) -> Result<Message, BoxError> {
    Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new("Test message content"))],
        SequenceNumber::new(sequence),
        clock,
    )
    .map_err(|e| Box::new(e) as BoxError)
}

/// Cleans up a test database.
///
/// # Errors
///
/// Returns an error if database cleanup fails.
pub fn cleanup_database(cluster: &ManagedCluster, db_name: &str) -> Result<(), BoxError> {
    cluster.drop_database(db_name)
}

/// Guard that ensures test database cleanup runs even if test panics.
///
/// Call [`Self::cleanup`] to surface cleanup errors in the test body.
pub struct CleanupGuard<'a> {
    cluster: &'a ManagedCluster,
    db_name: String,
}

impl<'a> CleanupGuard<'a> {
    pub const fn new(cluster: &'a ManagedCluster, db_name: String) -> Self {
        Self { cluster, db_name }
    }

    /// Explicitly cleanup the test database.
    ///
    /// # Errors
    ///
    /// Returns an error if database cleanup fails.
    pub fn cleanup(&self) -> Result<(), BoxError> {
        cleanup_database(self.cluster, &self.db_name)
    }
}

impl Drop for CleanupGuard<'_> {
    fn drop(&mut self) {
        drop(cleanup_database(self.cluster, &self.db_name));
    }
}

/// Expected audit context values for parameterized tests.
pub struct ExpectedAuditContext {
    pub correlation: Option<Uuid>,
    pub causation: Option<Uuid>,
    pub user: Option<Uuid>,
    pub session: Option<Uuid>,
}

impl ExpectedAuditContext {
    /// Creates an [`AuditContext`] from expected values.
    #[must_use]
    pub const fn to_audit_context(&self) -> AuditContext {
        let mut audit = AuditContext::empty();
        if let Some(id) = self.correlation {
            audit = audit.with_correlation_id(id);
        }
        if let Some(id) = self.causation {
            audit = audit.with_causation_id(id);
        }
        if let Some(id) = self.user {
            audit = audit.with_user_id(id);
        }
        if let Some(id) = self.session {
            audit = audit.with_session_id(id);
        }
        audit
    }
}

/// Helper struct for querying role from database.
#[derive(diesel::QueryableByName)]
pub struct RoleResult {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub role: String,
}

/// Inserts a conversation record to satisfy the foreign key constraint.
///
/// # Errors
///
/// Returns an error if connection or insert fails.
pub fn insert_conversation(
    cluster: &ManagedCluster,
    db_name: &str,
    conv_id: ConversationId,
) -> Result<(), BoxError> {
    let url = cluster.connection().database_url(db_name);
    let mut conn = PgConnection::establish(&url).map_err(|e| Box::new(e) as BoxError)?;

    diesel::sql_query(concat!(
        "INSERT INTO conversations (id, context, state, created_at, updated_at) ",
        "VALUES ($1, '{}', 'active', NOW(), NOW())",
    ))
    .bind::<diesel::sql_types::Uuid, _>(conv_id.into_inner())
    .execute(&mut conn)
    .map_err(|e| Box::new(e) as BoxError)?;

    Ok(())
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
///
/// # Errors
///
/// Returns an error if connection or query fails.
pub fn fetch_audit_log_for_message(
    cluster: &ManagedCluster,
    db_name: &str,
    message_id: uuid::Uuid,
) -> Result<Option<AuditLogRow>, BoxError> {
    let url = cluster.connection().database_url(db_name);
    let mut conn = PgConnection::establish(&url).map_err(|e| Box::new(e) as BoxError)?;

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
        Err(e) => Err(Box::new(e) as BoxError),
    }
}
