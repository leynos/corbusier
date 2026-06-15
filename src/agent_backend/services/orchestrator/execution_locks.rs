//! Per-session execution locks for in-process turn serialization.
//!
//! Per-session serialization is required because concurrent turns for the same
//! session race over shared session and conversation state, including message
//! ordering, domain-event sequence numbers, and task-status transitions. The
//! guard must span the full async turn-execution sequence from
//! `resolve_session` through `route_tool_calls` and persistence; releasing the
//! lock mid-turn and reacquiring it would let a second turn observe partial
//! state and violate the single-writer-per-session invariant.
//!
//! The lock table stores `Arc<Mutex<()>>` values as `Weak` references so entries
//! are dropped automatically when no turn holds or awaits the mutex, preventing
//! unbounded map growth. Callers acquire the guard through
//! `SessionExecutionLocks::lock()` and must hold the returned
//! `OwnedMutexGuard` for the duration of the turn; dropping it early silently
//! violates the invariant.

use crate::context::TenantId;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

type SessionKey = (TenantId, Uuid);
type SessionLock = Arc<Mutex<()>>;
type SessionLockRef = Weak<Mutex<()>>;

#[derive(Debug)]
pub(super) struct SessionExecutionLocks {
    locks: std::sync::Mutex<HashMap<SessionKey, SessionLockRef>>,
}

impl SessionExecutionLocks {
    pub(super) fn new() -> Self {
        Self {
            locks: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn lock_for(&self, tenant_id: TenantId, conversation_id: Uuid) -> SessionLock {
        let key = (tenant_id, conversation_id);
        let mut locks = match self.locks.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(existing) = locks.get(&key) {
            if let Some(upgraded) = existing.upgrade() {
                return upgraded;
            }
            locks.remove(&key);
        }

        let created = Arc::new(Mutex::new(()));
        locks.insert(key, Arc::downgrade(&created));
        created
    }

    pub(super) async fn lock(
        &self,
        tenant_id: TenantId,
        conversation_id: Uuid,
    ) -> OwnedMutexGuard<()> {
        self.lock_for(tenant_id, conversation_id).lock_owned().await
    }
}
