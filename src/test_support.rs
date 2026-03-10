//! Shared test utilities available to all `#[cfg(test)]` modules.

use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};

/// Creates a [`RequestContext`] with freshly generated identifiers.
pub fn test_request_ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}
