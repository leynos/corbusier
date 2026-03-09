//! Cross-cutting request context for tenant-scoped operations.

use super::ids::{CausationId, CorrelationId, SessionId, TenantId, UserId};

/// Cross-cutting request context for tenant-scoped operations.
///
/// Every repository and service method operating on tenant-owned data requires
/// a `RequestContext`. It carries the tenant identity, distributed tracing
/// identifiers, and the authenticated principal.
///
/// # Example
///
/// ```
/// use corbusier::context::{
///     CausationId, CorrelationId, RequestContext, SessionId, TenantId, UserId,
/// };
///
/// let ctx = RequestContext::new(
///     TenantId::new(),
///     CorrelationId::new(),
///     UserId::new(),
///     SessionId::new(),
/// )
/// .with_causation_id(CausationId::new());
///
/// assert!(ctx.causation_id().is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_field_names,
    reason = "fields mirror the domain concept names (tenant_id, correlation_id, etc.)"
)]
pub struct RequestContext {
    tenant_id: TenantId,
    correlation_id: CorrelationId,
    causation_id: Option<CausationId>,
    user_id: UserId,
    session_id: SessionId,
}

impl RequestContext {
    /// Creates a new request context with the required identifiers.
    #[must_use]
    pub const fn new(
        tenant_id: TenantId,
        correlation_id: CorrelationId,
        user_id: UserId,
        session_id: SessionId,
    ) -> Self {
        Self {
            tenant_id,
            correlation_id,
            causation_id: None,
            user_id,
            session_id,
        }
    }

    /// Sets the causation identifier.
    #[must_use]
    pub const fn with_causation_id(mut self, causation_id: CausationId) -> Self {
        self.causation_id = Some(causation_id);
        self
    }

    /// Returns the tenant identifier.
    #[must_use]
    pub const fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    /// Returns the correlation identifier.
    #[must_use]
    pub const fn correlation_id(&self) -> CorrelationId {
        self.correlation_id
    }

    /// Returns the causation identifier, if set.
    #[must_use]
    pub const fn causation_id(&self) -> Option<CausationId> {
        self.causation_id
    }

    /// Returns the user identifier.
    #[must_use]
    pub const fn user_id(&self) -> UserId {
        self.user_id
    }

    /// Returns the session identifier.
    #[must_use]
    pub const fn session_id(&self) -> SessionId {
        self.session_id
    }
}
