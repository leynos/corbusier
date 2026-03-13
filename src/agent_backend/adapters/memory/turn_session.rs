//! In-memory turn-session repository for orchestration tests.

use crate::agent_backend::{
    domain::{BackendId, TurnSession, TurnSessionId, TurnSessionStatus},
    ports::{
        SessionSlotArbitration, TurnSessionRepository, TurnSessionRepositoryError,
        TurnSessionRepositoryResult,
    },
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Default)]
struct InMemoryTurnSessionState {
    sessions: HashMap<TurnSessionId, TurnSession>,
    active_index: HashMap<(BackendId, Uuid), TurnSessionId>,
}

/// Thread-safe in-memory repository for turn sessions.
#[derive(Debug, Clone, Default)]
pub struct InMemoryTurnSessionRepository {
    state: Arc<RwLock<InMemoryTurnSessionState>>,
}

impl InMemoryTurnSessionRepository {
    /// Creates an empty in-memory turn-session repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all persisted sessions for assertions.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionRepositoryError::Persistence`] when the in-memory
    /// state lock cannot be acquired.
    pub fn all_sessions(&self) -> TurnSessionRepositoryResult<Vec<TurnSession>> {
        let state = self.state.read().map_err(|err| {
            TurnSessionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(state.sessions.values().cloned().collect())
    }

    fn reconcile_session_index(
        state: &mut InMemoryTurnSessionState,
        session: &TurnSession,
    ) -> TurnSessionRepositoryResult<()> {
        let key = (session.backend_id(), session.conversation_id());
        if session.status() == TurnSessionStatus::Active {
            let has_competing_active = state.active_index.get(&key).is_some_and(|existing_id| {
                *existing_id != session.id()
                    && state
                        .sessions
                        .get(existing_id)
                        .is_some_and(|stored| stored.status() == TurnSessionStatus::Active)
            });
            if has_competing_active {
                return Err(TurnSessionRepositoryError::active_session_conflict(
                    session.backend_id(),
                    session.conversation_id(),
                ));
            }

            state.sessions.insert(session.id(), session.clone());
            state.active_index.insert(key, session.id());
            return Ok(());
        }

        state.sessions.insert(session.id(), session.clone());
        if state
            .active_index
            .get(&key)
            .is_some_and(|id| *id == session.id())
        {
            state.active_index.remove(&key);
        }
        Ok(())
    }
}

#[async_trait]
impl TurnSessionRepository for InMemoryTurnSessionRepository {
    async fn arbitrate_session_slot(
        &self,
        backend_id: BackendId,
        conversation_id: Uuid,
        now: DateTime<Utc>,
    ) -> TurnSessionRepositoryResult<SessionSlotArbitration> {
        let mut state = self.state.write().map_err(|err| {
            TurnSessionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;

        let key = (backend_id, conversation_id);
        let Some(active_id) = state.active_index.get(&key).copied() else {
            return Ok(SessionSlotArbitration::Vacant);
        };

        let Some(existing) = state.sessions.get_mut(&active_id) else {
            state.active_index.remove(&key);
            return Ok(SessionSlotArbitration::Vacant);
        };

        if existing.status() != TurnSessionStatus::Active {
            state.active_index.remove(&key);
            return Ok(SessionSlotArbitration::Vacant);
        }

        if existing.is_expired_at(now) {
            existing.mark_expired(now);
            state.active_index.remove(&key);
            return Ok(SessionSlotArbitration::Expired);
        }

        Ok(SessionSlotArbitration::Reused(existing.clone()))
    }

    async fn find_active_session(
        &self,
        backend_id: BackendId,
        conversation_id: Uuid,
    ) -> TurnSessionRepositoryResult<Option<TurnSession>> {
        let state = self.state.read().map_err(|err| {
            TurnSessionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;

        let session = state
            .active_index
            .get(&(backend_id, conversation_id))
            .and_then(|id| state.sessions.get(id))
            .filter(|session| session.status() == TurnSessionStatus::Active)
            .cloned();

        Ok(session)
    }

    async fn upsert_session(&self, session: &TurnSession) -> TurnSessionRepositoryResult<()> {
        let mut state = self.state.write().map_err(|err| {
            TurnSessionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Self::reconcile_session_index(&mut state, session)
    }
}
