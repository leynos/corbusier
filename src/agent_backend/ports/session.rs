//! Session persistence port for turn orchestration.

use crate::agent_backend::domain::{BackendId, TurnSession};
use crate::context::RequestContext;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Result type for turn-session repository operations.
pub type TurnSessionRepositoryResult<T> = Result<T, TurnSessionRepositoryError>;

/// Session-slot arbitration result for a backend/conversation pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSlotArbitration {
    /// Existing active session remains reusable.
    Reused(TurnSession),
    /// No active session exists for the pair.
    Vacant,
    /// An active session existed but was expired during arbitration.
    Expired,
}

/// Repository contract for orchestration turn sessions.
#[async_trait]
pub trait TurnSessionRepository: Send + Sync {
    /// Atomically resolves active-session state for a backend/conversation
    /// pair within the tenant identified by [`RequestContext`].
    ///
    /// Adapters must perform the read/expiry transition in a single
    /// transaction so concurrent callers cannot observe torn state.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionRepositoryError`] on persistence failures.
    #[expect(
        clippy::too_many_arguments,
        reason = "tenant-scoped slot arbitration needs context, key, and clock inputs"
    )]
    async fn arbitrate_session_slot(
        &self,
        ctx: &RequestContext,
        backend_id: BackendId,
        conversation_id: Uuid,
        now: DateTime<Utc>,
    ) -> TurnSessionRepositoryResult<SessionSlotArbitration>;

    /// Finds the active session for a backend/conversation pair scoped by
    /// tenant.
    ///
    /// Returns `None` when there is no active session.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionRepositoryError`] on persistence failures.
    async fn find_active_session(
        &self,
        ctx: &RequestContext,
        backend_id: BackendId,
        conversation_id: Uuid,
    ) -> TurnSessionRepositoryResult<Option<TurnSession>>;

    /// Persists a session insert or update scoped by tenant.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionRepositoryError`] on persistence failures.
    async fn upsert_session(
        &self,
        ctx: &RequestContext,
        session: &TurnSession,
    ) -> TurnSessionRepositoryResult<()>;
}

/// Errors returned by turn-session repository adapters.
#[derive(Debug, Error)]
pub enum TurnSessionRepositoryError {
    /// Another active session already exists for backend/conversation pair.
    #[error("active session conflict for backend {backend_id} conversation {conversation_id}")]
    ActiveSessionConflict {
        /// Backend identifier for the conflicting active session.
        backend_id: BackendId,
        /// Conversation identifier for the conflicting active session.
        conversation_id: Uuid,
    },

    /// Persisted data could not be reconstructed into domain values.
    #[error("invalid persisted turn session data: {0}")]
    InvalidPersistedData(Arc<dyn std::error::Error + Send + Sync>),

    /// Domain data could not be converted for persistence.
    #[error("invalid turn session domain data: {0}")]
    InvalidDomainData(Arc<dyn std::error::Error + Send + Sync>),

    /// Persistence-layer failure.
    #[error("turn session persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl TurnSessionRepositoryError {
    /// Builds an active-session conflict error for backend/conversation pair.
    #[must_use]
    pub const fn active_session_conflict(backend_id: BackendId, conversation_id: Uuid) -> Self {
        Self::ActiveSessionConflict {
            backend_id,
            conversation_id,
        }
    }

    /// Wraps a persisted-data reconstruction failure.
    #[must_use]
    pub fn invalid_persisted_data(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::InvalidPersistedData(Arc::new(err))
    }

    /// Wraps an outbound domain-to-persistence conversion failure.
    #[must_use]
    pub fn invalid_domain_data(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::InvalidDomainData(Arc::new(err))
    }

    /// Wraps an infrastructure persistence failure.
    #[must_use]
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
