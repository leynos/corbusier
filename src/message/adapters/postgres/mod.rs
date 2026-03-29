//! `PostgreSQL` implementations of message subsystem ports using Diesel ORM.
//!
//! Provides production-grade persistence with JSONB storage for message content,
//! metadata, agent sessions, handoffs, and context snapshots, following
//! corbusier-design.md §6.2.3 and §4.2.1.1.

mod agent_session;
pub(crate) mod blocking_helpers;
mod context_snapshot;
mod conversion_helpers;
mod handoff;
mod sql_helpers;
pub(crate) mod tenant_tx;

pub use agent_session::PostgresAgentSessionRepository;
pub use context_snapshot::PostgresContextSnapshotAdapter;
pub use handoff::PostgresHandoffAdapter;

use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use super::audit_context::AuditContext;
use super::models::{MessageRow, NewMessage};
use super::schema::messages;
use crate::context::{RequestContext, TenantId};
use crate::message::{
    domain::{ConversationId, Message, MessageId, SequenceNumber},
    error::RepositoryError,
    ports::repository::{MessageRepository, RepositoryResult},
};

pub use blocking_helpers::PgPool;
use blocking_helpers::{get_conn_with, run_blocking_with};
pub(crate) use conversion_helpers::row_to_message;
use conversion_helpers::ser_err;
use sql_helpers::{InsertIds, insert_message, set_audit_context};
use tenant_tx::{FromTxError, TxError, ensure_tenant_exists, with_tenant_read_tx, with_tenant_tx};

// ---------------------------------------------------------------------------
// Error bridging for the shared transaction helper
// ---------------------------------------------------------------------------

impl FromTxError<Self> for RepositoryError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(e) => e,
            TxError::Diesel(e) => Self::database(e),
        }
    }
}

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

    async fn execute_inner<TxWrap, F, T>(
        &self,
        tenant_id: TenantId,
        tx_wrap: TxWrap,
        query_fn: F,
    ) -> RepositoryResult<T>
    where
        TxWrap: FnOnce(&mut PgConnection, uuid::Uuid, F) -> RepositoryResult<T> + Send + 'static,
        F: FnOnce(&mut PgConnection) -> RepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, RepositoryError::database)?;
                tx_wrap(&mut conn, tenant_id.into_inner(), query_fn)
            },
            RepositoryError::database,
        )
        .await
    }

    /// Executes a query inside a transaction with tenant context.
    async fn execute_query<F, T>(&self, tenant_id: TenantId, query_fn: F) -> RepositoryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> RepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let bootstrapping_tx = |conn: &mut PgConnection, tenant_uuid: uuid::Uuid, qfn: F| {
            with_tenant_tx(conn, tenant_uuid, |tx| {
                ensure_tenant_exists(tx, tenant_uuid).map_err(RepositoryError::database)?;
                qfn(tx)
            })
        };
        self.execute_inner(tenant_id, bootstrapping_tx, query_fn)
            .await
    }

    /// Executes a read-only query inside a transaction with tenant context.
    async fn execute_read_query<F, T>(
        &self,
        tenant_id: TenantId,
        query_fn: F,
    ) -> RepositoryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> RepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self.execute_inner(tenant_id, with_tenant_read_tx, query_fn)
            .await
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
        ctx: &RequestContext,
        message: &Message,
    ) -> RepositoryResult<()> {
        let audit_ctx = AuditContext::from(ctx);
        let tenant_id = ctx.tenant_id();
        let new_message = NewMessage::try_from_domain(message, tenant_id.into_inner())?;
        let msg_id = message.id();
        let conv_id = message.conversation_id();
        let seq_num = message.sequence_number();

        let ids = InsertIds {
            msg_id,
            conv_id,
            seq_num,
        };

        self.execute_query(tenant_id, move |tx_conn| {
            set_audit_context(tx_conn, &audit_ctx)?;
            insert_message(tx_conn, &new_message, &ids)?;
            Ok(())
        })
        .await
    }
}

#[async_trait]
impl MessageRepository for PostgresMessageRepository {
    async fn store(&self, ctx: &RequestContext, message: &Message) -> RepositoryResult<()> {
        let tenant_id = ctx.tenant_id();
        let new_message = NewMessage::try_from_domain(message, tenant_id.into_inner())?;
        let msg_id = message.id();
        let conv_id = message.conversation_id();
        let seq_num = message.sequence_number();

        self.execute_query(tenant_id, move |conn| {
            // Pre-check for duplicate message ID to provide semantic error
            let id_exists: i64 = messages::table
                .filter(messages::id.eq(msg_id.into_inner()))
                .filter(messages::tenant_id.eq(tenant_id.into_inner()))
                .count()
                .get_result(conn)
                .map_err(RepositoryError::database)?;

            if id_exists > 0 {
                return Err(RepositoryError::DuplicateMessage(msg_id));
            }

            // Pre-check for duplicate sequence number in conversation
            let seq_exists: i64 = messages::table
                .filter(messages::tenant_id.eq(tenant_id.into_inner()))
                .filter(messages::conversation_id.eq(conv_id.into_inner()))
                .filter(
                    messages::sequence_number.eq(i64::try_from(seq_num.value())
                        .map_err(|e| RepositoryError::serialization(e.to_string()))?),
                )
                .count()
                .get_result(conn)
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
            insert_message(conn, &new_message, &ids)
        })
        .await
    }

    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        id: MessageId,
    ) -> RepositoryResult<Option<Message>> {
        let tenant_id = ctx.tenant_id();
        let uuid = id.into_inner();

        self.execute_read_query(tenant_id, move |conn| {
            messages::table
                .filter(messages::id.eq(uuid))
                .filter(messages::tenant_id.eq(tenant_id.into_inner()))
                .select(MessageRow::as_select())
                .first::<MessageRow>(conn)
                .optional()
                .map_err(RepositoryError::database)?
                .map(row_to_message)
                .transpose()
        })
        .await
    }

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> RepositoryResult<Vec<Message>> {
        let tenant_id = ctx.tenant_id();
        let uuid = conversation_id.into_inner();

        self.execute_read_query(tenant_id, move |conn| {
            let rows = messages::table
                .filter(messages::tenant_id.eq(tenant_id.into_inner()))
                .filter(messages::conversation_id.eq(uuid))
                .order(messages::sequence_number.asc())
                .select(MessageRow::as_select())
                .load::<MessageRow>(conn)
                .map_err(RepositoryError::database)?;

            rows.into_iter().map(row_to_message).collect()
        })
        .await
    }

    async fn next_sequence_number(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> RepositoryResult<SequenceNumber> {
        let tenant_id = ctx.tenant_id();
        let uuid = conversation_id.into_inner();

        self.execute_read_query(tenant_id, move |conn| {
            let max_seq: Option<i64> = messages::table
                .filter(messages::tenant_id.eq(tenant_id.into_inner()))
                .filter(messages::conversation_id.eq(uuid))
                .select(diesel::dsl::max(messages::sequence_number))
                .first(conn)
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

    async fn exists(&self, ctx: &RequestContext, id: MessageId) -> RepositoryResult<bool> {
        let tenant_id = ctx.tenant_id();
        let uuid = id.into_inner();

        self.execute_read_query(tenant_id, move |conn| {
            let count: i64 = messages::table
                .filter(messages::id.eq(uuid))
                .filter(messages::tenant_id.eq(tenant_id.into_inner()))
                .count()
                .get_result(conn)
                .map_err(RepositoryError::database)?;

            Ok(count > 0)
        })
        .await
    }
}
