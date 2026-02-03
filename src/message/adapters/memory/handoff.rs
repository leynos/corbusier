//! In-memory implementation of the `AgentHandoffPort`.
//!
//! Provides a simple, thread-safe adapter for unit testing
//! without database dependencies.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use mockable::Clock;

use crate::message::{
    domain::{AgentSessionId, ConversationId, HandoffId, HandoffMetadata, HandoffParams},
    ports::handoff::{AgentHandoffPort, HandoffError, HandoffResult, InitiateHandoffParams},
};

/// In-memory implementation of [`AgentHandoffPort`].
///
/// Thread-safe via internal [`RwLock`]. Suitable for unit tests only.
#[derive(Debug, Clone)]
pub struct InMemoryHandoffAdapter<C: Clock + Send + Sync> {
    handoffs: Arc<RwLock<HashMap<HandoffId, HandoffMetadata>>>,
    clock: C,
}

impl<C: Clock + Send + Sync> InMemoryHandoffAdapter<C> {
    /// Creates a new adapter with the given clock.
    #[must_use]
    pub fn new(clock: C) -> Self {
        Self {
            handoffs: Arc::new(RwLock::new(HashMap::new())),
            clock,
        }
    }

    /// Returns the number of stored handoffs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.handoffs.read().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Returns `true` if no handoffs are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl<C: Clock + Send + Sync> AgentHandoffPort for InMemoryHandoffAdapter<C> {
    async fn initiate_handoff(
        &self,
        params: InitiateHandoffParams<'_>,
    ) -> HandoffResult<HandoffMetadata> {
        let handoff_params = HandoffParams::new(
            params.source_session.session_id,
            params.prior_turn_id,
            &params.source_session.agent_backend,
            params.target_agent,
        );
        let mut handoff = HandoffMetadata::new(handoff_params, &self.clock);

        if let Some(r) = params.reason {
            handoff = handoff.with_reason(r);
        }

        let mut guard = self
            .handoffs
            .write()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        guard.insert(handoff.handoff_id, handoff.clone());

        Ok(handoff)
    }

    async fn complete_handoff(
        &self,
        handoff_id: HandoffId,
        target_session_id: AgentSessionId,
    ) -> HandoffResult<HandoffMetadata> {
        let mut guard = self
            .handoffs
            .write()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        let handoff = guard
            .get(&handoff_id)
            .ok_or(HandoffError::NotFound(handoff_id))?;

        let completed = handoff.clone().complete(target_session_id, &self.clock);
        guard.insert(handoff_id, completed.clone());

        Ok(completed)
    }

    async fn cancel_handoff(
        &self,
        handoff_id: HandoffId,
        _reason: Option<&str>,
    ) -> HandoffResult<()> {
        let mut guard = self
            .handoffs
            .write()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        let handoff = guard
            .get(&handoff_id)
            .ok_or(HandoffError::NotFound(handoff_id))?;

        let cancelled = handoff.clone().cancel();
        guard.insert(handoff_id, cancelled);

        Ok(())
    }

    async fn find_handoff(&self, handoff_id: HandoffId) -> HandoffResult<Option<HandoffMetadata>> {
        let guard = self
            .handoffs
            .read()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        Ok(guard.get(&handoff_id).cloned())
    }

    async fn list_handoffs_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> HandoffResult<Vec<HandoffMetadata>> {
        let guard = self
            .handoffs
            .read()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        // Note: We don't have conversation_id in HandoffMetadata directly,
        // but we could filter by source_session's conversation.
        // For now, we return all handoffs (would need session lookup in real impl)
        let mut handoffs: Vec<HandoffMetadata> = guard.values().cloned().collect();

        // In a real implementation, we'd filter by conversation_id.
        // For testing, we store conversation_id association separately or
        // trust the caller to only query relevant conversations.
        let _ = conversation_id;

        handoffs.sort_by_key(|h| h.initiated_at);
        Ok(handoffs)
    }
}
