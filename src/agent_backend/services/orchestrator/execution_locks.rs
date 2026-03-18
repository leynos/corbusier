//! Per-session execution locks for in-process turn serialization.

use crate::agent_backend::domain::BackendId;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

type SessionKey = (BackendId, Uuid);
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

    fn lock_for(&self, backend_id: BackendId, conversation_id: Uuid) -> SessionLock {
        let key = (backend_id, conversation_id);
        let mut locks = match self.locks.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(existing) = locks.get(&key).and_then(Weak::upgrade) {
            return existing;
        }

        let created = Arc::new(Mutex::new(()));
        locks.insert(key, Arc::downgrade(&created));
        created
    }

    pub(super) async fn lock(
        &self,
        backend_id: BackendId,
        conversation_id: Uuid,
    ) -> OwnedMutexGuard<()> {
        self.lock_for(backend_id, conversation_id)
            .lock_owned()
            .await
    }
}
