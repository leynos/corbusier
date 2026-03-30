//! Shared HTTP API auth fixtures for integration tests.

use corbusier::{
    context::{CorrelationId, RequestContext, SessionId, TenantId, UserId},
    http_api::JwtClaims,
};
use jsonwebtoken::{EncodingKey, Header, encode};
use uuid::Uuid;

/// Consistent JWT and request-context fixture for HTTP API tests.
pub struct HttpApiAuth {
    secret: String,
    user_id: Uuid,
    tenant_id: Uuid,
    session_id: Uuid,
}

impl HttpApiAuth {
    /// Creates a new auth fixture.
    #[must_use]
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
        }
    }

    /// Returns the bearer token for this fixture.
    ///
    /// # Errors
    ///
    /// Returns an error when the claims cannot be encoded as a JWT.
    pub fn token(&self) -> Result<String, jsonwebtoken::errors::Error> {
        encode(
            &Header::default(),
            &JwtClaims::new(
                self.user_id,
                self.tenant_id,
                self.session_id,
                chrono::Utc::now().timestamp().saturating_add(3600),
            ),
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    /// Returns a request context matching the token claims.
    #[must_use]
    pub fn request_context(&self) -> RequestContext {
        RequestContext::new(
            TenantId::from_uuid(self.tenant_id),
            CorrelationId::new(),
            UserId::from_uuid(self.user_id),
            SessionId::from_uuid(self.session_id),
        )
    }
}
