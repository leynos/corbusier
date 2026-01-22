//! Shared test helpers for `PostgreSQL` integration tests.

use super::cluster::{BoxError, ManagedCluster, TemporaryDatabase};
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

/// Ensures the template database exists with the schema applied.
///
/// # Errors
///
/// Returns an error if template creation or migration fails.
pub async fn ensure_template(cluster: &ManagedCluster) -> Result<(), BoxError> {
    cluster
        .ensure_template_exists(TEMPLATE_DB, |db_name| {
            let url = cluster.connection().database_url(db_name);
            let mut conn =
                PgConnection::establish(&url).map_err(|err| Box::new(err) as BoxError)?;
            conn.batch_execute(CREATE_SCHEMA_SQL)
                .map_err(|err| Box::new(err) as BoxError)?;
            conn.batch_execute(ADD_CONSTRAINTS_SQL)
                .map_err(|err| Box::new(err) as BoxError)?;
            conn.batch_execute(ADD_AUDIT_TRIGGER_SQL)
                .map_err(|err| Box::new(err) as BoxError)?;
            Ok(())
        })
        .await
}

/// Creates a test database from template and returns a repository and cleanup guard.
///
/// # Errors
///
/// Returns an error if database creation or repository setup fails.
pub async fn setup_repository(
    cluster: &'static ManagedCluster,
) -> Result<(TemporaryDatabase, PostgresMessageRepository), BoxError> {
    let temp_db = cluster
        .temporary_database_from_template(&format!("test_{}", uuid::Uuid::new_v4()), TEMPLATE_DB)
        .await?;

    let url = temp_db.url();
    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|e| Box::new(e) as BoxError)?;

    let repo = PostgresMessageRepository::new(pool);
    Ok((temp_db, repo))
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
pub async fn insert_conversation(
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
pub async fn fetch_audit_log_for_message(
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
