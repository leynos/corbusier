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
    store: Arc<RwLock<HandoffStore>>,
    clock: C,
}

#[derive(Debug, Default)]
struct HandoffStore {
    handoffs: HashMap<HandoffId, HandoffMetadata>,
    conversations: HashMap<HandoffId, ConversationId>,
}

impl<C: Clock + Send + Sync> InMemoryHandoffAdapter<C> {
    /// Creates a new adapter with the given clock.
    #[must_use]
    pub fn new(clock: C) -> Self {
        Self {
            store: Arc::new(RwLock::new(HandoffStore::default())),
            clock,
        }
    }

    /// Returns the number of stored handoffs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.store
            .read()
            .map(|guard| guard.handoffs.len())
            .unwrap_or(0)
    }

    /// Returns `true` if no handoffs are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Helper method to update a handoff with terminal state validation.
    ///
    /// Acquires a write lock, fetches the handoff, validates it's not terminal,
    /// applies the transformation, and persists the result.
    fn update_handoff<F>(
        &self,
        handoff_id: HandoffId,
        target_status: crate::message::domain::HandoffStatus,
        transform: F,
    ) -> HandoffResult<HandoffMetadata>
    where
        F: FnOnce(HandoffMetadata) -> HandoffMetadata,
    {
        let mut guard = self
            .store
            .write()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        let handoff = guard
            .handoffs
            .get(&handoff_id)
            .ok_or(HandoffError::NotFound(handoff_id))?;

        if handoff.is_terminal() {
            return Err(HandoffError::invalid_transition(
                handoff.status,
                target_status,
            ));
        }

        let updated = transform(handoff.clone());
        guard.handoffs.insert(handoff_id, updated.clone());

        Ok(updated)
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
            .store
            .write()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        guard.handoffs.insert(handoff.handoff_id, handoff.clone());
        guard
            .conversations
            .insert(handoff.handoff_id, params.conversation_id);

        Ok(handoff)
    }

    async fn complete_handoff(
        &self,
        handoff_id: HandoffId,
        target_session_id: AgentSessionId,
    ) -> HandoffResult<HandoffMetadata> {
        let clock = &self.clock;
        self.update_handoff(
            handoff_id,
            crate::message::domain::HandoffStatus::Completed,
            |handoff| handoff.complete(target_session_id, clock),
        )
    }

    async fn cancel_handoff(
        &self,
        handoff_id: HandoffId,
        reason: Option<&str>,
    ) -> HandoffResult<()> {
        self.update_handoff(
            handoff_id,
            crate::message::domain::HandoffStatus::Cancelled,
            |handoff| handoff.cancel(reason),
        )?;
        Ok(())
    }

    async fn find_handoff(&self, handoff_id: HandoffId) -> HandoffResult<Option<HandoffMetadata>> {
        let guard = self
            .store
            .read()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        Ok(guard.handoffs.get(&handoff_id).cloned())
    }

    async fn list_handoffs_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> HandoffResult<Vec<HandoffMetadata>> {
        let guard = self
            .store
            .read()
            .map_err(|e| HandoffError::persistence(std::io::Error::other(e.to_string())))?;

        let mut handoffs: Vec<HandoffMetadata> = Vec::new();
        for (handoff_id, conv_id) in &guard.conversations {
            if *conv_id == conversation_id
                && let Some(handoff) = guard.handoffs.get(handoff_id)
            {
                handoffs.push(handoff.clone());
            }
        }

        handoffs.sort_by_key(|h| h.initiated_at);
        Ok(handoffs)
    }
}
