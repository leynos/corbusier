//! Per-session execution locks for in-process turn serialisation.
//!
//! [`SessionExecutionLocks`] provides a per-`(tenant_id, conversation_id)` async
//! mutex so that concurrent requests for the same conversation are serialised at
//! the orchestrator level.
//!
//! ## Why the lock must span the full turn
//!
//! A single turn involves four sequential, stateful operations: session
//! resolution, runtime execution, tool-call routing, and session persistence.
//! Each step reads and writes shared in-process session state that is only
//! committed to the database at the end of `persist_completed_turn`. If a
//! second turn for the same conversation were admitted between any two of those
//! steps, both turns would resolve the same session, execute against
//! inconsistent state, and then race to persist conflicting session updates.
//!
//! Database transactions guard persistence consistency, but they cannot prevent
//! the in-process race that arises between session resolution and the external
//! AI-backend call (which is inherently async and long-running). The lock
//! therefore covers the entire turn: from `resolve_session` through
//! `persist_completed_turn`. No two turns for the same conversation can be
//! in-flight simultaneously within a single process instance.

use crate::context::TenantId;
use std::collections::HashMap;

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
