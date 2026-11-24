//! Shared environment guards for integration tests.

use std::env;
use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, OnceLock};

static ENV_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

/// Guard that applies a scoped environment variable update.
pub struct EnvVarGuard {
    previous: Vec<(OsString, Option<OsString>)>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvVarGuard {
    /// Sets multiple environment variables for the guard lifetime.
    #[expect(
        clippy::disallowed_methods,
        reason = "mutex-serialized guard for the shared-cluster harness only: it starts PostgreSQL in the test process, which reads PG_* from the process environment, until pg-embed-setup-unpriv takes an injected environment (https://github.com/leynos/pg-embed-setup-unpriv/issues/441); every test of the environment helpers injects a reader instead (https://github.com/leynos/corbusier/issues/176)"
    )]
    pub fn set_many(changes: &[(OsString, Option<OsString>)]) -> Self {
        let lock = env_lock();
        let mut previous = Vec::with_capacity(changes.len());

        for (key, value) in changes {
            previous.push((key.clone(), env::var_os(key)));
            unsafe {
                // SAFETY: the global mutex serializes environment mutations in tests.
                match value {
                    Some(new_value) => env::set_var(key, new_value),
                    None => env::remove_var(key),
                }
            }
        }

        Self {
            previous,
            _lock: lock,
        }
    }
}

impl Drop for EnvVarGuard {
    #[expect(
        clippy::disallowed_methods,
        reason = "restores the variables set by `set_many`; goes with it when pg-embed-setup-unpriv takes an injected environment, https://github.com/leynos/pg-embed-setup-unpriv/issues/441"
    )]
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..) {
            unsafe {
                // SAFETY: the global mutex serializes environment mutations in tests.
                match value {
                    Some(previous) => env::set_var(&key, &previous),
                    None => env::remove_var(&key),
                }
            }
        }
    }
}

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
