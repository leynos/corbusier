//! `PostgreSQL` implementation of the conversation repository.

use crate::context::{RequestContext, TenantId};
use crate::message::adapters::{
    models::{ConversationRow, NewConversation},
    schema::conversations,
};
use crate::message::{
    domain::{Conversation, ConversationId, ConversationState},
    ports::conversation::{
        ConversationRepository, ConversationRepositoryError, ConversationRepositoryResult,
    },
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use super::{
    PgPool,
    blocking_helpers::{get_conn_with, run_blocking_with},
    tenant_tx::{FromTxError, TxError, with_tenant_tx},
};

impl FromTxError<Self> for ConversationRepositoryError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(error) => error,
            TxError::Diesel(error) => Self::persistence(error),
        }
    }
}

/// `PostgreSQL` implementation of [`ConversationRepository`].
#[derive(Debug, Clone)]
pub struct PostgresConversationRepository {
    pool: PgPool,
}

impl PostgresConversationRepository {
    /// Creates a new repository with the given connection pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn execute_query<F, T>(
        &self,
        tenant_id: TenantId,
        query_fn: F,
    ) -> ConversationRepositoryResult<T>
    where
        F: FnOnce(&mut PgConnection) -> ConversationRepositoryResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, ConversationRepositoryError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), query_fn)
            },
            ConversationRepositoryError::persistence,
        )
        .await
    }
}

fn row_to_conversation(row: &ConversationRow) -> ConversationRepositoryResult<Conversation> {
    let state = ConversationState::try_from(row.state.as_str())
        .map_err(|err| ConversationRepositoryError::persistence(std::io::Error::other(err)))?;
    Ok(Conversation::from_persisted(
        ConversationId::from_uuid(row.id),
        state,
        row.created_at,
        row.updated_at,
    ))
}

#[async_trait]
impl ConversationRepository for PostgresConversationRepository {
    async fn store(
        &self,
        ctx: &RequestContext,
        conversation: &Conversation,
    ) -> ConversationRepositoryResult<()> {
        let tenant_id = ctx.tenant_id();
        let conversation_id = conversation.id();
        let new_conversation = NewConversation::new(
            conversation_id.into_inner(),
            tenant_id.into_inner(),
            conversation.created_at(),
        );

        self.execute_query(tenant_id, move |conn| {
            // Use ON CONFLICT DO NOTHING for atomic insert-or-detect
            let inserted = diesel::insert_into(conversations::table)
                .values(&new_conversation)
                .on_conflict(conversations::id)
                .do_nothing()
                .execute(conn)
                .map_err(ConversationRepositoryError::persistence)?;

            if inserted == 0 {
                return Err(ConversationRepositoryError::DuplicateConversation(
                    conversation_id,
                ));
            }

            Ok(())
        })
        .await
    }

    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> ConversationRepositoryResult<Option<Conversation>> {
        let tenant_id = ctx.tenant_id();
        let uuid = conversation_id.into_inner();
        self.execute_query(tenant_id, move |conn| {
            conversations::table
                .filter(conversations::id.eq(uuid))
                .filter(conversations::tenant_id.eq(tenant_id.into_inner()))
                .select(ConversationRow::as_select())
                .first::<ConversationRow>(conn)
                .optional()
                .map_err(ConversationRepositoryError::persistence)?
                .as_ref()
                .map(row_to_conversation)
                .transpose()
        })
        .await
    }
}
