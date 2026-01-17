//! `PostgreSQL` implementation of the `MessageRepository` port using Diesel ORM.
//!
//! Provides production-grade persistence with JSONB storage for message content
//! and metadata, following corbusier-design.md §6.2.3.

mod blocking_helpers;
mod conversion_helpers;
mod sql_helpers;

use async_trait::async_trait;
use diesel::prelude::*;

use super::audit_context::AuditContext;
use super::models::{MessageRow, NewMessage};
use super::schema::messages;
use crate::message::{
    domain::{ConversationId, Message, MessageId, SequenceNumber},
    error::RepositoryError,
    ports::repository::{MessageRepository, RepositoryResult},
};

pub use blocking_helpers::PgPool;
use blocking_helpers::{get_conn, run_blocking};
pub(crate) use conversion_helpers::row_to_message;
use conversion_helpers::ser_err;
use sql_helpers::{InsertIds, insert_message, set_audit_context};

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
        let msg_id = message.id();
        let conv_id = message.conversation_id();
        let seq_num = message.sequence_number();

        let ids = InsertIds {
            msg_id,
            conv_id,
            seq_num,
        };

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;
            conn.transaction::<_, RepositoryError, _>(|tx_conn| {
                set_audit_context(tx_conn, &audit_ctx)?;
                insert_message(tx_conn, &new_message, &ids)?;
                Ok(())
            })
        })
        .await
    }
}

#[async_trait]
impl MessageRepository for PostgresMessageRepository {
    async fn store(&self, message: &Message) -> RepositoryResult<()> {
        let pool = self.pool.clone();
        let new_message = NewMessage::try_from_domain(message)?;
        let msg_id = message.id();
        let conv_id = message.conversation_id();
        let seq_num = message.sequence_number();

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            // Pre-check for duplicate message ID to provide semantic error
            let id_exists: i64 = messages::table
                .filter(messages::id.eq(msg_id.into_inner()))
                .count()
                .get_result(&mut conn)
                .map_err(RepositoryError::database)?;

            if id_exists > 0 {
                return Err(RepositoryError::DuplicateMessage(msg_id));
            }

            // Pre-check for duplicate sequence number in conversation
            let seq_exists: i64 = messages::table
                .filter(messages::conversation_id.eq(conv_id.into_inner()))
                .filter(
                    messages::sequence_number.eq(i64::try_from(seq_num.value())
                        .map_err(|e| RepositoryError::serialization(e.to_string()))?),
                )
                .count()
                .get_result(&mut conn)
                .map_err(RepositoryError::database)?;

            if seq_exists > 0 {
                return Err(RepositoryError::DuplicateSequence {
                    conversation_id: conv_id,
                    sequence: seq_num,
                });
            }

            let ids = InsertIds {
                msg_id,
                conv_id,
                seq_num,
            };
            insert_message(&mut conn, &new_message, &ids)
        })
        .await
    }

    async fn find_by_id(&self, id: MessageId) -> RepositoryResult<Option<Message>> {
        let pool = self.pool.clone();
        let uuid = id.into_inner();

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            messages::table
                .filter(messages::id.eq(uuid))
                .select(MessageRow::as_select())
                .first::<MessageRow>(&mut conn)
                .optional()
                .map_err(RepositoryError::database)?
                .map(row_to_message)
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

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            let rows = messages::table
                .filter(messages::conversation_id.eq(uuid))
                .order(messages::sequence_number.asc())
                .select(MessageRow::as_select())
                .load::<MessageRow>(&mut conn)
                .map_err(RepositoryError::database)?;

            rows.into_iter().map(row_to_message).collect()
        })
        .await
    }

    async fn next_sequence_number(
        &self,
        conversation_id: ConversationId,
    ) -> RepositoryResult<SequenceNumber> {
        let pool = self.pool.clone();
        let uuid = conversation_id.into_inner();

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

            let max_seq: Option<i64> = messages::table
                .filter(messages::conversation_id.eq(uuid))
                .select(diesel::dsl::max(messages::sequence_number))
                .first(&mut conn)
                .map_err(RepositoryError::database)?;

            let current = max_seq.unwrap_or(0);
            let next = current.checked_add(1).ok_or_else(|| {
                RepositoryError::serialization("sequence number overflow: maximum i64 reached")
            })?;
            let next_u64 = u64::try_from(next).map_err(ser_err)?;

            Ok(SequenceNumber::new(next_u64))
        })
        .await
    }

    async fn exists(&self, id: MessageId) -> RepositoryResult<bool> {
        let pool = self.pool.clone();
        let uuid = id.into_inner();

        run_blocking(move || {
            let mut conn = get_conn(&pool)?;

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
