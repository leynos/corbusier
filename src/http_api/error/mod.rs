//! HTTP error mapping and serialization for the API surface.
//!
//! This module translates domain and service-layer failures into the shared
//! `actix-v2a` error payload while preserving Corbusier-owned mapping logic and
//! route-level correlation IDs.

mod conversation;
mod task;
mod tool;

use actix_v2a::{Error as SharedApiError, ErrorCode, TRACE_ID_HEADER};
use actix_web::{
    HttpResponse, ResponseError,
    http::{StatusCode, header::HeaderValue},
};
use serde_json::{Value, json};
use std::fmt;

pub(crate) use self::{
    conversation::{map_conversation_repository_error, map_message_repository_error},
    task::{map_task_domain_error, map_task_repository_error},
    tool::map_tool_service_error,
};

use crate::message::{error::ValidationError, services::ConversationServiceError};
use crate::task::services::TaskLifecycleError;
use crate::tool_registry::services::ToolDiscoveryRoutingServiceError;

/// Shared API error type.
#[derive(Debug, Clone)]
pub struct ApiError {
    inner: SharedApiError,
    status: StatusCode,
}

impl ApiError {
    /// Creates a new API error.
    #[must_use]
    pub fn new(status: StatusCode, reason: &'static str, message: impl Into<String>) -> Self {
        Self {
            inner: shared_error(status, reason, message, None),
            status,
        }
    }

    /// Sets structured details on the error payload.
    #[must_use]
    pub fn with_details(mut self, details: Value) -> Self {
        let merged_details = match (self.inner.details().cloned(), details) {
            (Some(Value::Object(mut existing)), Value::Object(extra)) => {
                existing.extend(extra);
                Value::Object(existing)
            }
            (_, other) => other,
        };
        self.inner = self.inner.with_details(merged_details);
        self
    }

    /// Creates a `400 Bad Request` response.
    #[must_use]
    pub fn bad_request(reason: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, reason, message)
    }

    /// Creates a `401 Unauthorised` response.
    #[must_use]
    pub fn unauthorised(reason: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, reason, message)
    }

    /// Creates a `401 Unauthorised` response (US-spelled alias for API consistency).
    #[must_use]
    pub fn unauthorized(reason: &'static str, message: impl Into<String>) -> Self {
        Self::unauthorised(reason, message)
    }

    /// Creates a `404 Not Found` response.
    #[must_use]
    pub fn not_found(reason: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, reason, message)
    }

    /// Creates a `409 Conflict` response.
    #[must_use]
    pub fn conflict(reason: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, reason, message)
    }

    /// Creates a `500 Internal Server Error` response.
    #[must_use]
    pub fn internal() -> Self {
        Self {
            inner: SharedApiError::internal_static("Internal server error"),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Builds an HTTP response with the given request ID.
    #[must_use]
    pub fn into_response(
        self,
        _clock: &(impl mockable::Clock + ?Sized),
        request_id: impl Into<String>,
    ) -> HttpResponse {
        self.response_with_trace_id(request_id)
    }

    #[must_use]
    fn response_with_trace_id(&self, trace_id: impl Into<String>) -> HttpResponse {
        let payload = with_trace_id(self.inner.clone(), trace_id).redacted();
        let mut builder = HttpResponse::build(self.status);
        if let Some(header_value) = payload
            .trace_id()
            .and_then(|value| HeaderValue::from_str(value).ok())
        {
            builder.insert_header((TRACE_ID_HEADER, header_value));
        }

        builder.json(payload)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.inner.message())
    }
}

impl std::error::Error for ApiError {}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        self.response_with_trace_id("trace-unavailable")
    }
}

impl From<ValidationError> for ApiError {
    fn from(error: ValidationError) -> Self {
        Self::bad_request("validation_failed", error.to_string())
    }
}

impl From<ConversationServiceError> for ApiError {
    fn from(error: ConversationServiceError) -> Self {
        match error {
            ConversationServiceError::ConversationNotFound(conversation_id) => Self::not_found(
                "conversation_not_found",
                format!("conversation {conversation_id} was not found"),
            ),
            ConversationServiceError::ConversationRepository(repository_error) => {
                map_conversation_repository_error(repository_error)
            }
            ConversationServiceError::MessageRepository(repository_error) => {
                map_message_repository_error(repository_error)
            }
            ConversationServiceError::Validation(validation_error) => validation_error.into(),
            ConversationServiceError::RetryExhausted => {
                tracing::error!("conversation service retry exhausted");
                Self::internal()
            }
        }
    }
}

impl From<TaskLifecycleError> for ApiError {
    fn from(error: TaskLifecycleError) -> Self {
        match error {
            TaskLifecycleError::Domain(domain_error) => map_task_domain_error(&domain_error),
            TaskLifecycleError::InvalidState(parse_error) => {
                Self::bad_request("invalid_task_state", parse_error.to_string())
            }
            TaskLifecycleError::Repository(repository_error) => {
                map_task_repository_error(repository_error)
            }
        }
    }
}

impl From<ToolDiscoveryRoutingServiceError> for ApiError {
    fn from(error: ToolDiscoveryRoutingServiceError) -> Self {
        map_tool_service_error(error)
    }
}

fn shared_error(
    status: StatusCode,
    reason: &'static str,
    message: impl Into<String>,
    details: Option<Value>,
) -> SharedApiError {
    let error_code = error_code_for(status);
    let base = SharedApiError::try_new(error_code, message)
        .unwrap_or_else(|_| SharedApiError::internal_static("Internal server error"));
    let payload = match details {
        Some(Value::Object(mut object)) => {
            object.insert("reason".to_owned(), Value::String(reason.to_owned()));
            Value::Object(object)
        }
        Some(other) => json!({
            "reason": reason,
            "context": other,
        }),
        None => json!({ "reason": reason }),
    };

    base.with_details(payload)
}

const fn error_code_for(status: StatusCode) -> ErrorCode {
    match status {
        StatusCode::BAD_REQUEST => ErrorCode::InvalidRequest,
        StatusCode::UNAUTHORIZED => ErrorCode::Unauthorized,
        StatusCode::FORBIDDEN => ErrorCode::Forbidden,
        StatusCode::NOT_FOUND => ErrorCode::NotFound,
        StatusCode::CONFLICT => ErrorCode::Conflict,
        StatusCode::SERVICE_UNAVAILABLE => ErrorCode::ServiceUnavailable,
        _ => ErrorCode::InternalError,
    }
}

fn with_trace_id(error: SharedApiError, trace_id: impl Into<String>) -> SharedApiError {
    error.clone().try_with_trace_id(trace_id).unwrap_or(error)
}
