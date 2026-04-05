//! In-memory implementation of the conversation repository.

use crate::context::{RequestContext, TenantId};
use crate::message::{
    domain::{Conversation, ConversationId},
    ports::conversation::{
        ConversationRepository, ConversationRepositoryError, ConversationRepositoryResult,
    },
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe in-memory conversation repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryConversationRepository {
    conversations: Arc<RwLock<HashMap<TenantId, HashMap<ConversationId, Conversation>>>>,
}

impl InMemoryConversationRepository {
    /// Creates an empty repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ConversationRepository for InMemoryConversationRepository {
    async fn store(
        &self,
        ctx: &RequestContext,
        conversation: &Conversation,
    ) -> ConversationRepositoryResult<()> {
        let mut tenants = self.conversations.write().map_err(|err| {
            ConversationRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        let tenant_conversations = tenants.entry(ctx.tenant_id()).or_default();
        if tenant_conversations.contains_key(&conversation.id()) {
            return Err(ConversationRepositoryError::DuplicateConversation(
                conversation.id(),
            ));
        }
        tenant_conversations.insert(conversation.id(), conversation.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> ConversationRepositoryResult<Option<Conversation>> {
        let tenants = self.conversations.read().map_err(|err| {
            ConversationRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(tenants
            .get(&ctx.tenant_id())
            .and_then(|conversations| conversations.get(&conversation_id).cloned()))
    }
}
