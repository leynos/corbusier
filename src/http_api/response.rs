//! Shared HTTP response envelope types.

use actix_web::{HttpResponse, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Stable API version exposed by this adapter.
pub const API_VERSION: &str = "v1";

/// Shared metadata attached to every response.
#[derive(Debug, Clone, Serialize)]
pub struct ResponseMetadata {
    version: &'static str,
    request_id: String,
    timestamp: DateTime<Utc>,
}

impl ResponseMetadata {
    /// Creates metadata for a response.
    #[must_use]
    pub fn new(request_id: String) -> Self {
        Self {
            version: API_VERSION,
            request_id,
            timestamp: Utc::now(),
        }
    }
}

/// Error payload returned in failed responses.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    code: String,
    message: String,
}

impl ErrorPayload {
    /// Creates a new error payload.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Standard JSON envelope returned by the API.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    success: bool,
    data: Option<T>,
    error: Option<ErrorPayload>,
    metadata: ResponseMetadata,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    /// Creates a success envelope.
    #[must_use]
    pub fn success(data: T, request_id: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: ResponseMetadata::new(request_id),
        }
    }

    /// Creates an error envelope.
    #[must_use]
    pub fn error(error: ErrorPayload, request_id: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            metadata: ResponseMetadata::new(request_id),
        }
    }
}

/// Builds a successful JSON response with the shared envelope.
#[must_use]
pub fn json_success<T>(status: StatusCode, data: T, request_id: String) -> HttpResponse
where
    T: Serialize,
{
    HttpResponse::build(status).json(ApiResponse::success(data, request_id))
}

/// Builds a failed JSON response with the shared envelope.
#[must_use]
pub fn json_error(status: StatusCode, error: ErrorPayload, request_id: String) -> HttpResponse {
    HttpResponse::build(status).json(ApiResponse::<serde_json::Value>::error(error, request_id))
}
