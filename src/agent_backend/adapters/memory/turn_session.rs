//! In-memory turn-session repository for orchestration tests.

use crate::agent_backend::{
    domain::{
        BackendId, ReservedTurnSessionCreateParams, TurnSession, TurnSessionId, TurnSessionStatus,
    },
    ports::{
        SessionSlotArbitration, SessionSlotKey, SessionSlotReservation, TurnSessionRepository,
        TurnSessionRepositoryError, TurnSessionRepositoryResult,
    },
};
use crate::context::{RequestContext, TenantId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Default)]
struct InMemoryTurnSessionState {
    sessions: HashMap<TurnSessionId, TurnSession>,
    active_index: HashMap<(TenantId, BackendId, Uuid), TurnSessionId>,
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
        tenant_id: TenantId,
        session: &TurnSession,
    ) -> TurnSessionRepositoryResult<()> {
        let key = (tenant_id, session.backend_id(), session.conversation_id());
        if session.claims_slot() {
            let has_competing_active = state.active_index.get(&key).is_some_and(|existing_id| {
                *existing_id != session.id()
                    && state
                        .sessions
                        .get(existing_id)
                        .is_some_and(TurnSession::claims_slot)
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

    fn create_reservation_session(
        backend_id: BackendId,
        conversation_id: Uuid,
        now: DateTime<Utc>,
        ttl: chrono::Duration,
    ) -> TurnSessionRepositoryResult<TurnSession> {
        TurnSession::new_reserved(&ReservedTurnSessionCreateParams {
            id: TurnSessionId::new(),
            backend_id,
            conversation_id,
            ttl,
            now,
        })
        .map_err(TurnSessionRepositoryError::invalid_domain_data)
    }
}

#[async_trait]
impl TurnSessionRepository for InMemoryTurnSessionRepository {
    async fn arbitrate_session_slot(
        &self,
        ctx: &RequestContext,
        slot_reservation: SessionSlotReservation,
    ) -> TurnSessionRepositoryResult<SessionSlotArbitration> {
        let SessionSlotReservation { key, now, ttl } = slot_reservation;
        let SessionSlotKey {
            backend_id,
            conversation_id,
        } = key;
        let mut state = self.state.write().map_err(|err| {
            TurnSessionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;

        let index_key = (ctx.tenant_id(), backend_id, conversation_id);
        let mut prior_expired = None;

        if let Some(claimed_id) = state.active_index.get(&index_key).copied() {
            let Some(existing) = state.sessions.get_mut(&claimed_id) else {
                state.active_index.remove(&index_key);
                let reserved_session =
                    Self::create_reservation_session(backend_id, conversation_id, now, ttl)?;
                Self::reconcile_session_index(&mut state, ctx.tenant_id(), &reserved_session)?;
                return Ok(SessionSlotArbitration::Reserved {
                    reservation: reserved_session,
                    prior_expired,
                });
            };

            if existing.status() == TurnSessionStatus::Reserved {
                if existing.is_expired_at(now) {
                    existing.mark_expired(now);
                    state.active_index.remove(&index_key);
                    let reserved_session =
                        Self::create_reservation_session(backend_id, conversation_id, now, ttl)?;
                    Self::reconcile_session_index(&mut state, ctx.tenant_id(), &reserved_session)?;
                    return Ok(SessionSlotArbitration::Reserved {
                        reservation: reserved_session,
                        prior_expired,
                    });
                }
                return Err(TurnSessionRepositoryError::active_session_conflict(
                    backend_id,
                    conversation_id,
                ));
            }

            if existing.status() == TurnSessionStatus::Active && !existing.is_expired_at(now) {
                return Ok(SessionSlotArbitration::Reused(existing.clone()));
            }

            if existing.status() == TurnSessionStatus::Active {
                existing.mark_expired(now);
                prior_expired = Some(existing.clone());
            }

            state.active_index.remove(&index_key);
        }

        let reserved_session =
            Self::create_reservation_session(backend_id, conversation_id, now, ttl)?;
        Self::reconcile_session_index(&mut state, ctx.tenant_id(), &reserved_session)?;
        Ok(SessionSlotArbitration::Reserved {
            reservation: reserved_session,
            prior_expired,
        })
    }

    async fn find_active_session(
        &self,
        ctx: &RequestContext,
        key: SessionSlotKey,
    ) -> TurnSessionRepositoryResult<Option<TurnSession>> {
        let SessionSlotKey {
            backend_id,
            conversation_id,
        } = key;
        let state = self.state.read().map_err(|err| {
            TurnSessionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;

        let session = state
            .active_index
            .get(&(ctx.tenant_id(), backend_id, conversation_id))
            .and_then(|id| state.sessions.get(id))
            .filter(|session| session.status() == TurnSessionStatus::Active)
            .cloned();

        Ok(session)
    }

    async fn upsert_session(
        &self,
        ctx: &RequestContext,
        session: &TurnSession,
    ) -> TurnSessionRepositoryResult<()> {
        let mut state = self.state.write().map_err(|err| {
            TurnSessionRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Self::reconcile_session_index(&mut state, ctx.tenant_id(), session)
    }
}
