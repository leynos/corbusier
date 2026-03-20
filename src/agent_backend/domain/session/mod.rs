//! Turn-session domain model for agent backend orchestration, covering
//! per-conversation session continuity.

mod identifiers;
mod status;

pub use identifiers::{RuntimeSessionId, TurnSessionId};
pub use status::{ParseTurnSessionStatusError, TurnSessionStatus};

use super::BackendId;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Domain errors for turn-session construction and transitions.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TurnSessionDomainError {
    /// Runtime session IDs must not be empty.
    #[error("runtime session identifier must not be empty")]
    EmptyRuntimeSessionId,

    /// Session TTL must be positive.
    #[error("session ttl must be positive seconds, got {0}")]
    InvalidSessionTtl(i64),

    /// Turn counts cannot be recorded on expired sessions.
    #[error("cannot record turn on expired session")]
    RecordTurnOnExpiredSession,
}

/// Persisted data used to reconstruct [`TurnSession`] aggregates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedTurnSessionData {
    /// Unique session identifier.
    pub id: TurnSessionId,
    /// Backend that owns this session.
    pub backend_id: BackendId,
    /// Conversation identifier.
    pub conversation_id: Uuid,
    /// Backend-native session identifier.
    pub runtime_session_id: RuntimeSessionId,
    /// Lifecycle status.
    pub status: TurnSessionStatus,
    /// TTL in seconds used for sliding expiry.
    pub ttl_seconds: i64,
    /// Session creation time.
    pub started_at: DateTime<Utc>,
    /// Last successful turn time.
    pub last_used_at: DateTime<Utc>,
    /// Current expiry deadline.
    pub expires_at: DateTime<Utc>,
    /// Terminal timestamp when expired.
    pub ended_at: Option<DateTime<Utc>>,
    /// Number of turns executed in this session.
    pub turn_count: u64,
}

/// Parameters for creating a new active [`TurnSession`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSessionCreateParams {
    /// Backend that owns this session.
    pub backend_id: BackendId,
    /// Conversation identifier.
    pub conversation_id: Uuid,
    /// Backend-native runtime session identifier.
    pub runtime_session_id: RuntimeSessionId,
    /// Sliding session TTL.
    pub ttl: Duration,
    /// Current timestamp used for initial session times.
    pub now: DateTime<Utc>,
}

/// Parameters for creating a reserved [`TurnSession`] slot claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedTurnSessionCreateParams {
    /// Unique identifier for the reservation row.
    pub id: TurnSessionId,
    /// Backend that owns this session slot.
    pub backend_id: BackendId,
    /// Conversation identifier.
    pub conversation_id: Uuid,
    /// Sliding session TTL.
    pub ttl: Duration,
    /// Current timestamp used for initial session times.
    pub now: DateTime<Utc>,
}

/// Session aggregate used by agent turn orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnSession {
    id: TurnSessionId,
    backend_id: BackendId,
    conversation_id: Uuid,
    runtime_session_id: RuntimeSessionId,
    status: TurnSessionStatus,
    ttl_seconds: i64,
    started_at: DateTime<Utc>,
    last_used_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    turn_count: u64,
}

impl TurnSession {
    /// Creates a new active session.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionDomainError`] when the runtime session identifier
    /// is empty or the provided TTL is not positive.
    pub fn new(params: TurnSessionCreateParams) -> Result<Self, TurnSessionDomainError> {
        Self::new_with_status(TurnSessionId::new(), TurnSessionStatus::Active, params)
    }

    /// Creates a new reserved session-slot claim.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionDomainError`] when the reservation placeholder or
    /// provided TTL is invalid.
    pub fn new_reserved(
        params: &ReservedTurnSessionCreateParams,
    ) -> Result<Self, TurnSessionDomainError> {
        let runtime_session_id =
            RuntimeSessionId::new(format!("reservation:{}", params.id.into_inner()))?;
        Self::new_with_status(
            params.id,
            TurnSessionStatus::Reserved,
            TurnSessionCreateParams {
                backend_id: params.backend_id,
                conversation_id: params.conversation_id,
                runtime_session_id,
                ttl: params.ttl,
                now: params.now,
            },
        )
    }

    fn new_with_status(
        id: TurnSessionId,
        status: TurnSessionStatus,
        params: TurnSessionCreateParams,
    ) -> Result<Self, TurnSessionDomainError> {
        let ttl_seconds = params.ttl.num_seconds();
        if ttl_seconds <= 0 {
            return Err(TurnSessionDomainError::InvalidSessionTtl(ttl_seconds));
        }

        Ok(Self {
            id,
            backend_id: params.backend_id,
            conversation_id: params.conversation_id,
            runtime_session_id: params.runtime_session_id,
            status,
            ttl_seconds,
            started_at: params.now,
            last_used_at: params.now,
            expires_at: params.now + Duration::seconds(ttl_seconds),
            ended_at: None,
            turn_count: 0,
        })
    }

    /// Reconstructs a session aggregate from persisted data.
    #[must_use]
    pub fn from_persisted(data: PersistedTurnSessionData) -> Self {
        Self {
            id: data.id,
            backend_id: data.backend_id,
            conversation_id: data.conversation_id,
            runtime_session_id: data.runtime_session_id,
            status: data.status,
            ttl_seconds: data.ttl_seconds,
            started_at: data.started_at,
            last_used_at: data.last_used_at,
            expires_at: data.expires_at,
            ended_at: data.ended_at,
            turn_count: data.turn_count,
        }
    }

    /// Returns the session identifier.
    #[must_use]
    pub const fn id(&self) -> TurnSessionId {
        self.id
    }

    /// Returns the owning backend identifier.
    #[must_use]
    pub const fn backend_id(&self) -> BackendId {
        self.backend_id
    }

    /// Returns the conversation identifier.
    #[must_use]
    pub const fn conversation_id(&self) -> Uuid {
        self.conversation_id
    }

    /// Returns the backend-native runtime session identifier.
    #[must_use]
    pub fn runtime_session_id(&self) -> &str {
        self.runtime_session_id.as_str()
    }

    /// Returns the backend-native runtime session identifier wrapper.
    #[must_use]
    pub const fn runtime_session_handle(&self) -> &RuntimeSessionId {
        &self.runtime_session_id
    }

    /// Returns the current lifecycle status.
    #[must_use]
    pub const fn status(&self) -> TurnSessionStatus {
        self.status
    }

    /// Returns `true` when the session is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.status, TurnSessionStatus::Active)
    }

    /// Returns `true` when the session currently claims the slot.
    #[must_use]
    pub const fn claims_slot(&self) -> bool {
        matches!(
            self.status,
            TurnSessionStatus::Active | TurnSessionStatus::Reserved
        )
    }

    /// Returns session TTL in seconds.
    #[must_use]
    pub const fn ttl_seconds(&self) -> i64 {
        self.ttl_seconds
    }

    /// Returns the session start timestamp.
    #[must_use]
    pub const fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// Returns the most recent usage timestamp.
    #[must_use]
    pub const fn last_used_at(&self) -> DateTime<Utc> {
        self.last_used_at
    }

    /// Returns the expiry deadline.
    #[must_use]
    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    /// Returns the terminal timestamp if expired.
    #[must_use]
    pub const fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    /// Returns the number of turns executed in the session.
    #[must_use]
    pub const fn turn_count(&self) -> u64 {
        self.turn_count
    }

    /// Returns `true` when the session is expired at `now`.
    #[must_use]
    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        !self.is_active() || now >= self.expires_at
    }

    /// Records a successful turn and extends expiry using a sliding TTL window.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionDomainError::RecordTurnOnExpiredSession`] when the
    /// session is not active.
    pub fn record_turn(&mut self, now: DateTime<Utc>) -> Result<(), TurnSessionDomainError> {
        if self.status != TurnSessionStatus::Active {
            return Err(TurnSessionDomainError::RecordTurnOnExpiredSession);
        }
        self.turn_count = self.turn_count.saturating_add(1);
        self.last_used_at = now;
        self.expires_at = now + Duration::seconds(self.ttl_seconds);
        Ok(())
    }

    /// Marks the session expired.
    pub const fn mark_expired(&mut self, now: DateTime<Utc>) {
        self.status = TurnSessionStatus::Expired;
        self.ended_at = Some(now);
    }

    /// Promotes a reserved session-slot claim to an active runtime session.
    ///
    /// # Errors
    ///
    /// Returns [`TurnSessionDomainError::RecordTurnOnExpiredSession`] when the
    /// session is not currently reserved.
    pub fn activate(
        &mut self,
        runtime_session_id: RuntimeSessionId,
    ) -> Result<(), TurnSessionDomainError> {
        if self.status != TurnSessionStatus::Reserved {
            return Err(TurnSessionDomainError::RecordTurnOnExpiredSession);
        }

        self.runtime_session_id = runtime_session_id;
        self.status = TurnSessionStatus::Active;
        Ok(())
    }
}
