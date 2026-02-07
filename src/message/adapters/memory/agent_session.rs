//! In-memory implementation of the `AgentSessionRepository` port.
//!
//! Provides a simple, thread-safe repository for unit testing
//! without database dependencies.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::message::{
    domain::{AgentSession, AgentSessionId, AgentSessionState, ConversationId},
    ports::agent_session::{AgentSessionRepository, SessionError, SessionResult},
};

/// In-memory implementation of [`AgentSessionRepository`].
///
/// Thread-safe via internal [`RwLock`]. Suitable for unit tests only.
#[derive(Debug, Default, Clone)]
pub struct InMemoryAgentSessionRepository {
    sessions: Arc<RwLock<HashMap<AgentSessionId, AgentSession>>>,
}

impl InMemoryAgentSessionRepository {
    /// Creates an empty repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of stored sessions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sessions.read().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Returns `true` if no sessions are stored.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn upsert_with_check<F>(&self, session: &AgentSession, validate: F) -> SessionResult<()>
    where
        F: FnOnce(&HashMap<AgentSessionId, AgentSession>) -> SessionResult<()>,
    {
        let mut guard = self
            .sessions
            .write()
            .map_err(|e| SessionError::persistence(std::io::Error::other(e.to_string())))?;

        validate(&guard)?;

        guard.insert(session.session_id, session.clone());
        Ok(())
    }
}

#[async_trait]
impl AgentSessionRepository for InMemoryAgentSessionRepository {
    async fn store(&self, session: &AgentSession) -> SessionResult<()> {
        let session_id = session.session_id;
        self.upsert_with_check(session, |sessions| {
            if sessions.contains_key(&session_id) {
                return Err(SessionError::Duplicate(session_id));
            }

            Ok(())
        })
    }

    async fn update(&self, session: &AgentSession) -> SessionResult<()> {
        let session_id = session.session_id;
        self.upsert_with_check(session, |sessions| {
            if !sessions.contains_key(&session_id) {
                return Err(SessionError::NotFound(session_id));
            }

            Ok(())
        })
    }

    async fn find_by_id(&self, id: AgentSessionId) -> SessionResult<Option<AgentSession>> {
        let guard = self
            .sessions
            .read()
            .map_err(|e| SessionError::persistence(std::io::Error::other(e.to_string())))?;

        Ok(guard.get(&id).cloned())
    }

    async fn find_active_for_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> SessionResult<Option<AgentSession>> {
        let guard = self
            .sessions
            .read()
            .map_err(|e| SessionError::persistence(std::io::Error::other(e.to_string())))?;

        Ok(guard
            .values()
            .find(|s| s.conversation_id == conversation_id && s.state == AgentSessionState::Active)
            .cloned())
    }

    async fn find_by_conversation(
        &self,
        conversation_id: ConversationId,
    ) -> SessionResult<Vec<AgentSession>> {
        let guard = self
            .sessions
            .read()
            .map_err(|e| SessionError::persistence(std::io::Error::other(e.to_string())))?;

        let mut sessions: Vec<AgentSession> = guard
            .values()
            .filter(|s| s.conversation_id == conversation_id)
            .cloned()
            .collect();

        // Sort by start time
        sessions.sort_by_key(|s| s.started_at);

        Ok(sessions)
    }
}
