//! Session persistence port for turn orchestration.

use crate::agent_backend::domain::{BackendId, TurnSession};
use crate::context::RequestContext;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use thiserror::Error;
use uuid::Uuid;

/// Result type for turn-session repository operations.
pub type TurnSessionRepositoryResult<T> = Result<T, TurnSessionRepositoryError>;

/// Composite key identifying an active session slot for a
/// backend/conversation pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionSlotKey {
    /// Backend whose session slot is being queried.
    pub backend_id: BackendId,
    /// Conversation whose session slot is being queried.
    pub conversation_id: Uuid,
}

impl SessionSlotKey {
    /// Creates a new session-slot key.
    #[must_use]
    pub const fn new(backend_id: BackendId, conversation_id: Uuid) -> Self {
        Self {
            backend_id,
            conversation_id,
        }
    }
}

/// Parameters for slot arbitration in the session repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionSlotReservation {
    /// Composite key identifying the session slot to arbitrate.
    pub key: SessionSlotKey,
    /// Timestamp used for expiry evaluation and reservation creation.
    pub now: DateTime<Utc>,
    /// TTL assigned to a newly created reservation row.
    pub ttl: Duration,
}

impl SessionSlotReservation {
    /// Creates a new session-slot reservation request.
    #[must_use]
    pub const fn new(key: SessionSlotKey, now: DateTime<Utc>, ttl: Duration) -> Self {
        Self { key, now, ttl }
    }
}

/// Session-slot arbitration result for a backend/conversation pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionSlotArbitration {
    /// Existing active session remains reusable.
    Reused(TurnSession),
    /// Session slot was durably reserved for a new runtime session.
    Reserved {
        /// Persisted reservation row for the claimed slot.
        reservation: TurnSession,
        /// Expired session that should be torn down after the reservation
        /// commits.
        prior_expired: Option<TurnSession>,
    },
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
    async fn arbitrate_session_slot(
        &self,
        ctx: &RequestContext,
        reservation: SessionSlotReservation,
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
        key: SessionSlotKey,
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
    #[error("invalid persisted turn session data for '{field}': {reason}")]
    InvalidPersistedData {
        /// Name of the field that failed reconstruction.
        field: &'static str,
        /// Human-readable reason.
        reason: String,
    },

    /// Domain data could not be converted for persistence.
    #[error("invalid turn session domain data for '{field}': {reason}")]
    InvalidDomainData {
        /// Name of the field that failed serialisation.
        field: &'static str,
        /// Human-readable reason.
        reason: String,
    },

    /// Persistence-layer failure (infrastructure error, not inspectable by callers).
    #[error("turn session storage failure: {message}")]
    StorageFailure {
        /// Human-readable description of the infrastructure fault.
        message: String,
    },
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
    pub fn invalid_persisted_data(field: &'static str, reason: impl std::fmt::Display) -> Self {
        Self::InvalidPersistedData {
            field,
            reason: reason.to_string(),
        }
    }

    /// Wraps an outbound domain-to-persistence conversion failure.
    #[must_use]
    pub fn invalid_domain_data(field: &'static str, reason: impl std::fmt::Display) -> Self {
        Self::InvalidDomainData {
            field,
            reason: reason.to_string(),
        }
    }

    /// Wraps an infrastructure persistence failure.
    #[must_use]
    pub fn storage_failure(err: impl std::fmt::Display) -> Self {
        Self::StorageFailure {
            message: err.to_string(),
        }
    }
}
