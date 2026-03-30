//! Authentication and request-context extraction for the HTTP API.

use super::{error::ApiError, state::ApiState};
use actix_web::{FromRequest, HttpRequest, dev::Payload, web};
use chrono::Utc;
use futures::future::{Ready, ready};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};

/// JWT claims accepted by the initial HTTP API release.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwtClaims {
    /// Subject claim carrying the user identifier.
    pub sub: String,
    /// Tenant identifier claim.
    pub tenant_id: String,
    /// Session identifier claim.
    pub session_id: String,
    /// Expiry timestamp.
    pub exp: i64,
    /// Optional coarse role claim.
    pub role: Option<String>,
    /// Optional tenant-kind claim.
    pub tenant_kind: Option<String>,
}

impl JwtClaims {
    /// Creates test claims with the provided identifiers.
    #[must_use]
    pub fn new(user_id: Uuid, tenant_id: Uuid, session_id: Uuid, exp: i64) -> Self {
        Self {
            sub: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            session_id: session_id.to_string(),
            exp,
            role: None,
            tenant_kind: Some("user".to_owned()),
        }
    }
}

/// HS256 bearer-token authenticator.
#[derive(Clone)]
pub struct BearerTokenAuthenticator {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl BearerTokenAuthenticator {
    /// Creates a new authenticator from the shared secret.
    #[must_use]
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        Self {
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
            validation,
        }
    }

    fn authenticate_token(&self, token: &str) -> Result<RequestContext, ApiError> {
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|_| ApiError::unauthorized("invalid bearer token"))?;
        let claims = token_data.claims;
        let tenant_kind = claims.tenant_kind.as_deref().unwrap_or("user");
        if tenant_kind != "user" {
            return Err(ApiError::unauthorized("unsupported tenant kind"));
        }

        let tenant_id = parse_uuid_claim(&claims.tenant_id, "tenant_id")?;
        let user_id = parse_uuid_claim(&claims.sub, "sub")?;
        let session_id = parse_uuid_claim(&claims.session_id, "session_id")?;
        Ok(RequestContext::new(
            TenantId::from_uuid(tenant_id),
            CorrelationId::new(),
            UserId::from_uuid(user_id),
            SessionId::from_uuid(session_id),
        ))
    }
}

fn parse_uuid_claim(value: &str, claim_name: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value)
        .map_err(|_| ApiError::unauthorized(format!("invalid {claim_name} claim")))
}

fn extract_bearer_token(request: &HttpRequest) -> Result<&str, ApiError> {
    let header = request
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .ok_or_else(|| ApiError::unauthorized("missing bearer token"))?;
    let header_value = header
        .to_str()
        .map_err(|_| ApiError::unauthorized("invalid authorization header"))?;
    let Some(token) = header_value.strip_prefix("Bearer ") else {
        return Err(ApiError::unauthorized("invalid authorization header"));
    };
    if token.trim().is_empty() {
        return Err(ApiError::unauthorized("missing bearer token"));
    }
    Ok(token)
}

/// Request extractor carrying the authenticated request context.
#[derive(Debug, Clone)]
pub struct AuthenticatedRequestContext(pub RequestContext);

impl AuthenticatedRequestContext {
    /// Returns the shared request identifier.
    #[must_use]
    pub fn request_id(&self) -> String {
        self.0.correlation_id().to_string()
    }

    /// Returns the current request context.
    #[must_use]
    pub const fn context(&self) -> &RequestContext {
        &self.0
    }
}

impl FromRequest for AuthenticatedRequestContext {
    type Error = ApiError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = request
            .app_data::<web::Data<ApiState>>()
            .ok_or_else(|| ApiError::internal("API state not configured"))
            .and_then(|state| {
                let token = extract_bearer_token(request)?;
                state.authenticator.authenticate_token(token)
            })
            .map(Self);
        ready(result)
    }
}

/// Returns a short-lived expiration timestamp for tests or examples.
#[must_use]
pub fn default_test_expiry() -> i64 {
    Utc::now().timestamp().saturating_add(3600)
}
