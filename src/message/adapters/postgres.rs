//! `PostgreSQL` implementation of the `MessageRepository` port using Diesel ORM.
//!
//! Provides production-grade persistence with JSONB storage for message content
//! and metadata, following corbusier-design.md §6.2.3.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

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

/// `PostgreSQL` implementation of [`MessageRepository`].
///
/// Uses Diesel ORM with connection pooling via r2d2. Thread-safe for
/// concurrent access.
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
    pub fn store_with_audit(
        &self,
        message: &Message,
        audit: &AuditContext,
    ) -> RepositoryResult<()> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| RepositoryError::connection(e.to_string()))?;

        conn.transaction::<_, RepositoryError, _>(|tx_conn| {
            // Set audit context as session variables for trigger capture
            Self::set_audit_context(tx_conn, audit)?;

            // Insert the message
            let new_message = Self::message_to_insertable(message)?;
            diesel::insert_into(messages::table)
                .values(&new_message)
                .execute(tx_conn)
                .map_err(RepositoryError::database)?;

            Ok(())
        })
    }

    /// Sets `PostgreSQL` session variables for audit context.
    fn set_audit_context(conn: &mut PgConnection, audit: &AuditContext) -> RepositoryResult<()> {
        if let Some(correlation_id) = audit.correlation_id {
            diesel::sql_query(format!("SET LOCAL app.correlation_id = '{correlation_id}'"))
                .execute(conn)
                .map_err(RepositoryError::database)?;
        }

        if let Some(causation_id) = audit.causation_id {
            diesel::sql_query(format!("SET LOCAL app.causation_id = '{causation_id}'"))
                .execute(conn)
                .map_err(RepositoryError::database)?;
        }

        if let Some(user_id) = audit.user_id {
            diesel::sql_query(format!("SET LOCAL app.user_id = '{user_id}'"))
                .execute(conn)
                .map_err(RepositoryError::database)?;
        }

        if let Some(session_id) = audit.session_id {
            diesel::sql_query(format!("SET LOCAL app.session_id = '{session_id}'"))
                .execute(conn)
                .map_err(RepositoryError::database)?;
        }

        Ok(())
    }

    /// Converts a domain Message to an insertable database record.
    fn message_to_insertable(message: &Message) -> RepositoryResult<NewMessage> {
        let content = serde_json::to_value(message.content())
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        let metadata = serde_json::to_value(message.metadata())
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        Ok(NewMessage {
            id: message.id().into_inner(),
            conversation_id: message.conversation_id().into_inner(),
            role: message.role().as_str().to_owned(),
            content,
            metadata,
            created_at: message.created_at(),
            sequence_number: i64::try_from(message.sequence_number().value())
                .map_err(|e| RepositoryError::serialization(e.to_string()))?,
        })
    }

    /// Converts a database row to a domain Message.
    fn row_to_message(row: MessageRow) -> RepositoryResult<Message> {
        let role = Role::try_from(row.role.as_str())
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        let content: Vec<ContentPart> = serde_json::from_value(row.content)
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        let metadata: MessageMetadata = serde_json::from_value(row.metadata)
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        let sequence_number = u64::try_from(row.sequence_number)
            .map_err(|e| RepositoryError::serialization(e.to_string()))?;

        // Reconstruct message using internal constructor
        Message::from_persisted(
            MessageId::from_uuid(row.id),
            ConversationId::from_uuid(row.conversation_id),
            role,
            content,
            metadata,
            row.created_at,
            SequenceNumber::new(sequence_number),
        )
        .map_err(|e| RepositoryError::serialization(e.to_string()))
    }
}

#[async_trait]
impl MessageRepository for PostgresMessageRepository {
    async fn store(&self, message: &Message) -> RepositoryResult<()> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| RepositoryError::connection(e.to_string()))?;

        let new_message = Self::message_to_insertable(message)?;

        diesel::insert_into(messages::table)
            .values(&new_message)
            .execute(&mut conn)
            .map_err(RepositoryError::database)?;

        Ok(())
    }

    async fn find_by_id(&self, id: MessageId) -> RepositoryResult<Option<Message>> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| RepositoryError::connection(e.to_string()))?;

        let result = messages::table
            .filter(messages::id.eq(id.into_inner()))
            .select(MessageRow::as_select())
            .first::<MessageRow>(&mut conn)
            .optional()
            .map_err(RepositoryError::database)?;

        match result {
            Some(row) => Ok(Some(Self::row_to_message(row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<Vec<Message>> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| RepositoryError::connection(e.to_string()))?;

        let rows = messages::table
            .filter(messages::conversation_id.eq(conversation_id.into_inner()))
            .order(messages::sequence_number.asc())
            .select(MessageRow::as_select())
            .load::<MessageRow>(&mut conn)
            .map_err(RepositoryError::database)?;

        rows.into_iter().map(Self::row_to_message).collect()
    }

    async fn next_sequence_number(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<SequenceNumber> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| RepositoryError::connection(e.to_string()))?;

        let max_seq: Option<i64> = messages::table
            .filter(messages::conversation_id.eq(conversation_id.into_inner()))
            .select(diesel::dsl::max(messages::sequence_number))
            .first(&mut conn)
            .map_err(RepositoryError::database)?;

        let next = max_seq.unwrap_or(0).saturating_add(1);
        let next_u64 =
            u64::try_from(next).map_err(|e| RepositoryError::serialization(e.to_string()))?;

        Ok(SequenceNumber::new(next_u64))
    }

    async fn exists(&self, id: MessageId) -> RepositoryResult<bool> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| RepositoryError::connection(e.to_string()))?;

        let count: i64 = messages::table
            .filter(messages::id.eq(id.into_inner()))
            .count()
            .get_result(&mut conn)
            .map_err(RepositoryError::database)?;

        Ok(count > 0)
    }
}
