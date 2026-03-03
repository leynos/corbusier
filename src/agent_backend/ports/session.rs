//! Session persistence port for turn orchestration.

use crate::agent_backend::domain::{BackendId, TurnSession};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Result type for turn-session repository operations.
pub type TurnSessionRepositoryResult<T> = Result<T, TurnSessionRepositoryError>;

/// Repository contract for orchestration turn sessions.
#[async_trait]
pub trait TurnSessionRepository: Send + Sync {
    /// Finds the active session for a backend/conversation pair.
    ///
    /// Returns `None` when there is no active session.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionRepositoryError`] on persistence failures.
    async fn find_active_session(
        &self,
        backend_id: BackendId,
        conversation_id: Uuid,
    ) -> TurnSessionRepositoryResult<Option<TurnSession>>;

    /// Persists a session insert or update.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionRepositoryError`] on persistence failures.
    async fn upsert_session(&self, session: &TurnSession) -> TurnSessionRepositoryResult<()>;
}

/// Errors returned by turn-session repository adapters.
#[derive(Debug, Error)]
pub enum TurnSessionRepositoryError {
    /// Persisted data could not be reconstructed into domain values.
    #[error("invalid persisted turn session data: {0}")]
    InvalidPersistedData(Arc<dyn std::error::Error + Send + Sync>),

    /// Persistence-layer failure.
    #[error("turn session persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl TurnSessionRepositoryError {
    /// Wraps a persisted-data reconstruction failure.
    #[must_use]
    pub fn invalid_persisted_data(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::InvalidPersistedData(Arc::new(err))
    }

    /// Wraps an infrastructure persistence failure.
    #[must_use]
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
