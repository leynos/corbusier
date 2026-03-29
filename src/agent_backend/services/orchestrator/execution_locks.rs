//! Per-session execution locks for in-process turn serialization.

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
