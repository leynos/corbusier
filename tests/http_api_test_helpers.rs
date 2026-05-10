//! Shared HTTP API auth fixtures for integration tests.

use actix_web::{http::header, test as actix_test};
use corbusier::{
    context::{CorrelationId, RequestContext, SessionId, TenantId, UserId},
    http_api::JwtClaims,
    tool_registry::{
        adapters::InMemoryMcpServerHost,
        domain::{McpServerId, McpServerRegistration, McpToolDefinition, McpTransport},
        services::RegisterMcpServerRequest,
    },
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

const JWT_TTL_SECONDS: i64 = 86_400;

/// A bearer token string produced by or used with [`HttpApiAuth`].
pub struct BearerToken(
    /// The encoded bearer token value.
    pub String,
);

impl BearerToken {
    /// Returns the token as a string slice suitable for an authorization header.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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
                chrono::Utc::now()
                    .timestamp()
                    .saturating_add(JWT_TTL_SECONDS),
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
    fn encode_custom_claims(
        &self,
        claims: &CustomJwtClaims,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        self.encode_claims(claims)
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
        self.encode_custom_claims(&CustomJwtClaims {
            sub: self.user_id.to_string(),
            tenant_id: self.tenant_id.to_string(),
            session_id: self.session_id.to_string(),
            exp: chrono::Utc::now()
                .timestamp()
                .saturating_add(JWT_TTL_SECONDS),
            role: None,
            tenant_kind: Some(tenant_kind.into()),
        })
    }

    /// Returns a token with non-UUID strings for identifiers.
    ///
    /// # Errors
    ///
    /// Returns an error when the claims cannot be encoded as a JWT.
    pub fn token_with_invalid_uuids(&self) -> Result<String, jsonwebtoken::errors::Error> {
        self.encode_custom_claims(&CustomJwtClaims {
            sub: "not-a-uuid".to_owned(),
            tenant_id: "also-not-a-uuid".to_owned(),
            session_id: "still-not-a-uuid".to_owned(),
            exp: chrono::Utc::now()
                .timestamp()
                .saturating_add(JWT_TTL_SECONDS),
            role: None,
            tenant_kind: Some("user".to_owned()),
        })
    }

    /// Returns an already-expired token.
    ///
    /// # Errors
    ///
    /// Returns an error when the claims cannot be encoded as a JWT.
    pub fn expired_token(&self) -> Result<String, jsonwebtoken::errors::Error> {
        self.encode_custom_claims(&CustomJwtClaims {
            sub: self.user_id.to_string(),
            tenant_id: self.tenant_id.to_string(),
            session_id: self.session_id.to_string(),
            exp: chrono::Utc::now().timestamp().saturating_sub(3600),
            role: None,
            tenant_kind: Some("user".to_owned()),
        })
    }
}

/// Returns the named field from a JSON object used in HTTP API tests.
///
/// # Panics
///
/// Panics when `value` does not contain `key`.
#[must_use]
pub fn required_field<'a>(value: &'a Value, key: &str) -> &'a Value {
    let Some(field) = value.get(key) else {
        panic!("expected field `{key}` to be present");
    };
    field
}

/// Returns the named string field from a JSON object used in HTTP API tests.
///
/// # Panics
///
/// Panics when `value` does not contain `key` or the field is not a string.
#[must_use]
pub fn required_str_field<'a>(value: &'a Value, key: &str) -> &'a str {
    let Some(field) = required_field(value, key).as_str() else {
        panic!("expected field `{key}` to be a string");
    };
    field
}

/// Adds a bearer token to a test request.
#[must_use]
pub fn with_bearer(request: actix_test::TestRequest, token: &str) -> actix_test::TestRequest {
    request.insert_header((header::AUTHORIZATION, format!("Bearer {token}")))
}

/// Asserts the standard v1 metadata envelope.
///
/// # Panics
///
/// Panics if the top-level `metadata` field is missing, if the `version` field
/// is not equal to `"v1"`, or if `request_id` or `timestamp` are missing or
/// not strings. This delegates the field presence checks to
/// [`required_field`].
pub fn assert_v1_metadata(body: &Value) {
    let metadata = required_field(body, "metadata");
    assert_eq!(required_field(metadata, "version"), "v1");
    assert!(required_field(metadata, "request_id").is_string());
    assert!(required_field(metadata, "timestamp").is_string());
}

/// Asserts the shared `actix-v2a` error contract.
///
/// # Panics
///
/// Panics if `body` does not contain the expected top-level shared error
/// fields.
pub fn assert_shared_error(body: &Value, expected_code: &str) {
    assert_eq!(required_field(body, "code"), expected_code);
    assert!(required_field(body, "message").is_string());
    assert!(required_field(body, "traceId").is_string());
}

/// Registers and starts the standard `file_tools` MCP server used by HTTP API tests.
///
/// # Errors
///
/// Returns an error if server registration, startup, discovery, or host seeding fails.
pub async fn bootstrap_file_tools_server<
    RegisterF,
    RegisterFut,
    StartF,
    StartFut,
    DiscoverF,
    DiscoverFut,
>(
    host: &InMemoryMcpServerHost,
    register: RegisterF,
    start: StartF,
    discover: DiscoverF,
) -> Result<(), eyre::Report>
where
    RegisterF: FnOnce(RegisterMcpServerRequest) -> RegisterFut,
    RegisterFut: std::future::Future<Output = Result<McpServerRegistration, eyre::Report>>,
    StartF: FnOnce(McpServerId) -> StartFut,
    StartFut: std::future::Future<Output = Result<(), eyre::Report>>,
    DiscoverF: FnOnce(McpServerId) -> DiscoverFut,
    DiscoverFut: std::future::Future<Output = Result<(), eyre::Report>>,
{
    let server = register(RegisterMcpServerRequest::new(
        "file_tools",
        McpTransport::stdio("echo")?,
    ))
    .await?;
    host.set_tool_catalog(
        server.name().clone(),
        vec![McpToolDefinition::new(
            "read_file",
            "Read a file from disk",
            json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "required": ["path"]
            }),
        )?],
    )
    .map_err(|error| eyre::eyre!("tool catalog should be set: {error}"))?;
    host.set_tool_call_result(
        server.name().clone(),
        "read_file",
        json!({"content": "hello from tool"}),
    )
    .map_err(|error| eyre::eyre!("tool call result should be set: {error}"))?;
    start(server.id()).await?;
    discover(server.id()).await?;
    Ok(())
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

#[cfg(test)]
mod tests {
    //! Smoke tests for the shared HTTP API helper surface.

    use super::{
        BearerToken, HttpApiAuth, assert_shared_error, assert_v1_metadata, required_field,
        required_str_field, with_bearer,
    };
    use actix_web::test as actix_test;
    use serde_json::json;

    #[test]
    fn helper_surface_is_exercised() {
        let auth = HttpApiAuth::new("test-secret");
        let _ = auth.token().expect("token encoding should succeed");
        let _ = auth
            .encode_claims(&json!({ "sub": "user" }))
            .expect("custom claims encoding should succeed");
        let _ = auth
            .token_with_tenant_kind("user")
            .expect("tenant kind token encoding should succeed");
        let _ = auth
            .token_with_invalid_uuids()
            .expect("invalid UUID token encoding should succeed");
        let _ = auth
            .expired_token()
            .expect("expired token encoding should succeed");

        let payload = json!({ "message": "hello" });
        let field = required_field(&payload, "message");
        assert_eq!(required_str_field(&payload, "message"), "hello");
        assert_eq!(field, "hello");
        assert_v1_metadata(&json!({
            "metadata": {
                "version": "v1",
                "request_id": "req-123",
                "timestamp": "2026-04-01T00:00:00Z"
            }
        }));
        assert_shared_error(
            &json!({
                "code": "unauthorized",
                "message": "missing bearer token",
                "traceId": "trace-123",
            }),
            "unauthorized",
        );
        let request = with_bearer(actix_test::TestRequest::get(), "token-value");
        let _request = request.to_request();
        let token = BearerToken("token-value".to_owned());
        assert_eq!(token.as_str(), "token-value");
    }
}
