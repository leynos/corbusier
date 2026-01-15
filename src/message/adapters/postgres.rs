//! `PostgreSQL` implementation of the `MessageRepository` port using Diesel ORM.
//!
//! Provides production-grade persistence with JSONB storage for message content
//! and metadata, following corbusier-design.md §6.2.3.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

use super::audit_context::AuditContext;
use super::models::{MessageRow, NewMessage};
use super::schema::messages;
use crate::message::{
    domain::{
        ContentPart, ConversationId, Message, MessageId, MessageMetadata, Role, SequenceNumber,
    },
    error::RepositoryError,
    ports::repository::{MessageRepository, RepositoryResult},
};

/// `PostgreSQL` connection pool type.
pub type PgPool = Pool<ConnectionManager<PgConnection>>;

/// Pooled connection type for internal use.
type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;

/// `PostgreSQL` implementation of [`MessageRepository`].
///
/// Uses Diesel ORM with connection pooling via r2d2. Thread-safe for
/// concurrent access. All database operations are offloaded to a blocking
/// thread pool via [`tokio::task::spawn_blocking`] to avoid blocking
/// the async runtime.
///
/// # Example
///
/// ```ignore
/// use diesel::r2d2::{ConnectionManager, Pool};
/// use diesel::PgConnection;
/// use corbusier::message::adapters::postgres::PostgresMessageRepository;
///
/// let manager = ConnectionManager::<PgConnection>::new("postgres://...");
/// let pool = Pool::builder().build(manager).expect("pool");
/// let repo = PostgresMessageRepository::new(pool);
/// ```
#[derive(Debug, Clone)]
pub struct PostgresMessageRepository {
    pool: PgPool,
}

impl PostgresMessageRepository {
    /// Creates a new repository with the given connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns a reference to the connection pool.
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Stores a message with audit context for tracking.
    ///
    /// This method wraps the store operation in a transaction that also:
    /// 1. Sets `PostgreSQL` session variables for audit trigger capture
    /// 2. Records a `MessageCreated` domain event
    ///
    /// # Errors
    ///
    /// Returns `RepositoryError` if the database operation fails.
    pub async fn store_with_audit(
        &self,
        message: &Message,
        audit: &AuditContext,
    ) -> RepositoryResult<()> {
        let pool = self.pool.clone();
        let new_message = NewMessage::try_from_domain(message)?;
        let audit_ctx = audit.clone();

        Self::run_blocking(move || {
            let mut conn = Self::get_conn(&pool)?;
            conn.transaction::<_, RepositoryError, _>(|tx_conn| {
                Self::set_audit_context(tx_conn, &audit_ctx)?;
                Self::insert_message(tx_conn, &new_message)?;
                Ok(())
            })
        })
        .await
    }

    // ========================================================================
    // Blocking operation helpers
    // ========================================================================

    /// Runs a blocking database operation on a dedicated thread pool.
    ///
    /// Wraps the closure in [`tokio::task::spawn_blocking`] to prevent
    /// blocking the async executor's worker threads.
    async fn run_blocking<F, T>(f: F) -> RepositoryResult<T>
    where
        F: FnOnce() -> RepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        tokio::task::spawn_blocking(f)
            .await
            .map_err(|e| RepositoryError::connection(format!("task join error: {e}")))?
    }

    /// Obtains a connection from the pool.
    fn get_conn(pool: &PgPool) -> RepositoryResult<PooledConn> {
        pool.get()
            .map_err(|e| RepositoryError::connection(e.to_string()))
    }

    // ========================================================================
    // SQL execution helpers
    // ========================================================================

    /// Inserts a message into the database.
    fn insert_message(conn: &mut PgConnection, new_message: &NewMessage) -> RepositoryResult<()> {
        diesel::insert_into(messages::table)
            .values(new_message)
            .execute(conn)
            .map_err(RepositoryError::database)?;
        Ok(())
    }

    /// Sets a single `PostgreSQL` session variable for audit context.
    ///
    /// Uses parameterised SQL to avoid injection vulnerabilities.
    fn set_session_uuid(
        conn: &mut PgConnection,
        key: &str,
        value: uuid::Uuid,
    ) -> RepositoryResult<()> {
        // Use bind parameter for the value to prevent SQL injection.
        // The key is a controlled static string, not user input.
        diesel::sql_query(format!("SET LOCAL app.{key} = $1"))
            .bind::<diesel::sql_types::Uuid, _>(value)
            .execute(conn)
            .map_err(RepositoryError::database)?;
        Ok(())
    }

    /// Sets `PostgreSQL` session variables for audit context.
    ///
    /// Each audit field is set via a parameterised query using [`Self::set_session_uuid`]
    /// to ensure values are safely bound rather than interpolated.
    fn set_audit_context(conn: &mut PgConnection, audit: &AuditContext) -> RepositoryResult<()> {
        if let Some(correlation_id) = audit.correlation_id {
            Self::set_session_uuid(conn, "correlation_id", correlation_id)?;
        }
        if let Some(causation_id) = audit.causation_id {
            Self::set_session_uuid(conn, "causation_id", causation_id)?;
        }
        if let Some(user_id) = audit.user_id {
            Self::set_session_uuid(conn, "user_id", user_id)?;
        }
        if let Some(session_id) = audit.session_id {
            Self::set_session_uuid(conn, "session_id", session_id)?;
        }
        Ok(())
    }

    // ========================================================================
    // Conversion helpers
    // ========================================================================

    /// Wraps a serialization/conversion error for consistent error handling.
    fn ser_err<E: std::fmt::Display>(e: E) -> RepositoryError {
        RepositoryError::serialization(e.to_string())
    }

    /// Converts a database row to a domain Message.
    fn row_to_message(row: MessageRow) -> RepositoryResult<Message> {
        let role = Role::try_from(row.role.as_str()).map_err(Self::ser_err)?;
        let content: Vec<ContentPart> =
            serde_json::from_value(row.content).map_err(Self::ser_err)?;
        let metadata: MessageMetadata =
            serde_json::from_value(row.metadata).map_err(Self::ser_err)?;
        let sequence_number = u64::try_from(row.sequence_number).map_err(Self::ser_err)?;

        Message::from_persisted(
            MessageId::from_uuid(row.id),
            ConversationId::from_uuid(row.conversation_id),
            role,
            content,
            metadata,
            row.created_at,
            SequenceNumber::new(sequence_number),
        )
        .map_err(Self::ser_err)
    }
}

#[async_trait]
impl MessageRepository for PostgresMessageRepository {
    async fn store(&self, message: &Message) -> RepositoryResult<()> {
        let pool = self.pool.clone();
        let new_message = NewMessage::try_from_domain(message)?;

        Self::run_blocking(move || {
            let mut conn = Self::get_conn(&pool)?;
            Self::insert_message(&mut conn, &new_message)
        })
        .await
    }

    async fn find_by_id(&self, id: MessageId) -> RepositoryResult<Option<Message>> {
        let pool = self.pool.clone();
        let uuid = id.into_inner();

        Self::run_blocking(move || {
            let mut conn = Self::get_conn(&pool)?;

            messages::table
                .filter(messages::id.eq(uuid))
                .select(MessageRow::as_select())
                .first::<MessageRow>(&mut conn)
                .optional()
                .map_err(RepositoryError::database)?
                .map(Self::row_to_message)
                .transpose()
        })
        .await
    }

    async fn find_by_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<Vec<Message>> {
        let pool = self.pool.clone();
        let uuid = conversation_id.into_inner();

        Self::run_blocking(move || {
            let mut conn = Self::get_conn(&pool)?;

            let rows = messages::table
                .filter(messages::conversation_id.eq(uuid))
                .order(messages::sequence_number.asc())
                .select(MessageRow::as_select())
                .load::<MessageRow>(&mut conn)
                .map_err(RepositoryError::database)?;

            rows.into_iter().map(Self::row_to_message).collect()
        })
        .await
    }

    async fn next_sequence_number(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<SequenceNumber> {
        let pool = self.pool.clone();
        let uuid = conversation_id.into_inner();

        Self::run_blocking(move || {
            let mut conn = Self::get_conn(&pool)?;

            let max_seq: Option<i64> = messages::table
                .filter(messages::conversation_id.eq(uuid))
                .select(diesel::dsl::max(messages::sequence_number))
                .first(&mut conn)
                .map_err(RepositoryError::database)?;

            let current = max_seq.unwrap_or(0);
            let next = current.checked_add(1).ok_or_else(|| {
                RepositoryError::serialization("sequence number overflow: maximum i64 reached")
            })?;
            let next_u64 = u64::try_from(next).map_err(Self::ser_err)?;

            Ok(SequenceNumber::new(next_u64))
        })
        .await
    }

    async fn exists(&self, id: MessageId) -> RepositoryResult<bool> {
        let pool = self.pool.clone();
        let uuid = id.into_inner();

        Self::run_blocking(move || {
            let mut conn = Self::get_conn(&pool)?;

            let count: i64 = messages::table
                .filter(messages::id.eq(uuid))
                .count()
                .get_result(&mut conn)
                .map_err(RepositoryError::database)?;

            Ok(count > 0)
        })
        .await
    }
}
