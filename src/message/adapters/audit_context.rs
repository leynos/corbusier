//! Audit context for tracking request origin through the persistence layer.
//!
//! Context is propagated to `PostgreSQL` via session settings for trigger capture,
//! enabling comprehensive audit trails per corbusier-design.md §6.2.3.4.

use uuid::Uuid;

/// Audit context for a database operation.
///
/// Contains tracking identifiers that are captured by database triggers
/// into the `audit_logs` table. These identifiers enable distributed tracing
/// and causation chain reconstruction.
///
/// # Example
///
/// ```
/// use corbusier::message::adapters::audit_context::AuditContext;
/// use uuid::Uuid;
///
/// let context = AuditContext::empty()
///     .with_correlation_id(Uuid::new_v4())
///     .with_user_id(Uuid::new_v4());
///
/// assert!(context.correlation_id.is_some());
/// assert!(context.user_id.is_some());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditContext {
    /// Correlation ID for distributed tracing.
    ///
    /// Links all operations within a single user request across services.
    pub correlation_id: Option<Uuid>,

    /// Causation ID linking to the triggering event.
    ///
    /// Points to the domain event that caused this operation.
    pub causation_id: Option<Uuid>,

    /// User ID of the actor performing the operation.
    pub user_id: Option<Uuid>,

    /// Session ID for the current user session.
    pub session_id: Option<Uuid>,
}

impl AuditContext {
    /// Creates an empty audit context with no identifiers set.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            correlation_id: None,
            causation_id: None,
            user_id: None,
            session_id: None,
        }
    }

    /// Sets the correlation ID for distributed tracing.
    #[must_use]
    pub const fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Sets the causation ID linking to a triggering event.
    #[must_use]
    pub const fn with_causation_id(mut self, id: Uuid) -> Self {
        self.causation_id = Some(id);
        self
    }

    /// Sets the user ID of the actor.
    #[must_use]
    pub const fn with_user_id(mut self, id: Uuid) -> Self {
        self.user_id = Some(id);
        self
    }

    /// Sets the session ID.
    #[must_use]
    pub const fn with_session_id(mut self, id: Uuid) -> Self {
        self.session_id = Some(id);
        self
    }

    /// Returns `true` if all context fields are `None`.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.correlation_id.is_none()
            && self.causation_id.is_none()
            && self.user_id.is_none()
            && self.session_id.is_none()
    }
}
