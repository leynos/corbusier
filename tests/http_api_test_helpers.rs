//! Shared HTTP API auth fixtures for integration tests.

#![expect(
    dead_code,
    reason = "helpers are used selectively across integration test suites"
)]
#![allow(
    unfulfilled_lint_expectations,
    reason = "module is compiled for multiple test suites with varying usage"
)]

use corbusier::{
    context::{CorrelationId, RequestContext, SessionId, TenantId, UserId},
    http_api::JwtClaims,
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
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

    /// Encodes arbitrary claims into a JWT for testing edge cases.
    ///
    /// # Errors
    ///
    /// Returns an error when the claims cannot be encoded as a JWT.
    pub fn encode_claims<C: Serialize>(
        &self,
        claims: &C,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    /// Encodes custom claims into a JWT.
    #[expect(clippy::too_many_arguments, reason = "JWT claims have many fields")]
    fn encode_custom_claims(
        &self,
        sub: String,
        tenant_id: String,
        session_id: String,
        exp: i64,
        tenant_kind: Option<String>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        self.encode_claims(&CustomJwtClaims {
            sub,
            tenant_id,
            session_id,
            exp,
            role: None,
            tenant_kind,
        })
    }

    /// Returns a token with the specified `tenant_kind` claim.
    ///
    /// # Errors
    ///
    /// Returns an error when the claims cannot be encoded as a JWT.
    pub fn token_with_tenant_kind(
        &self,
        tenant_kind: impl Into<String>,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        self.encode_custom_claims(
            self.user_id.to_string(),
            self.tenant_id.to_string(),
            self.session_id.to_string(),
            chrono::Utc::now().timestamp().saturating_add(3600),
            Some(tenant_kind.into()),
        )
    }

    /// Returns a token with non-UUID strings for identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when the claims cannot be encoded as a JWT.
    pub fn token_with_invalid_uuids(&self) -> Result<String, jsonwebtoken::errors::Error> {
        self.encode_custom_claims(
            "not-a-uuid".to_owned(),
            "also-not-a-uuid".to_owned(),
            "still-not-a-uuid".to_owned(),
            chrono::Utc::now().timestamp().saturating_add(3600),
            Some("user".to_owned()),
        )
    }

    /// Returns an already-expired token.
    ///
    /// # Errors
    ///
    /// Returns an error when the claims cannot be encoded as a JWT.
    pub fn expired_token(&self) -> Result<String, jsonwebtoken::errors::Error> {
        self.encode_custom_claims(
            self.user_id.to_string(),
            self.tenant_id.to_string(),
            self.session_id.to_string(),
            chrono::Utc::now().timestamp().saturating_sub(3600),
            Some("user".to_owned()),
        )
    }
}

/// Custom JWT claims for constructing edge-case tokens.
#[derive(Debug, Clone, Serialize)]
struct CustomJwtClaims {
    sub: String,
    tenant_id: String,
    session_id: String,
    exp: i64,
    role: Option<String>,
    tenant_kind: Option<String>,
}
