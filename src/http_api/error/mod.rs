//! HTTP error mapping and serialization for the API surface.
//!
//! This module translates domain and service-layer failures into the versioned
//! HTTP error envelope returned by the API. Helpers such as
//! `map_task_domain_error`, `map_tool_domain_error`, and
//! `map_tool_catalog_error` map typed failures onto stable status codes and
//! response bodies, for example validation failures to `400 Bad Request`,
//! missing resources to `404 Not Found`, and unexpected infrastructure
//! failures to `500 Internal Server Error`.
//!
//! Correlation IDs are threaded through [`ApiError`] so handlers can preserve
//! request identifiers in both logs and serialized error payloads. The key
//! invariant is that handler-supplied request IDs are reused when available,
//! while fallback responses generate a fresh identifier only when no
//! correlation ID was attached earlier in the request lifecycle.

mod conversation;
mod task;
mod tool;

use super::response::{ErrorPayload, json_error};
use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use mockable::DefaultClock;
use std::fmt;
use uuid::Uuid;

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
    status: StatusCode,
    code: &'static str,
    message: String,
    request_id: Option<String>,
}

impl ApiError {
    /// Creates a new API error.
    #[must_use]
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            request_id: None,
        }
    }

    /// Sets the request ID for correlation.
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Creates a `400 Bad Request` response.
    #[must_use]
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    /// Creates a `401 Unauthorised` response.
    #[must_use]
    pub fn unauthorised(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorised", message)
    }

    /// Creates a `401 Unauthorised` response (US-spelled alias for API consistency).
    #[must_use]
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::unauthorised(message)
    }

    /// Creates a `404 Not Found` response.
    #[must_use]
    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    /// Creates a `409 Conflict` response.
    #[must_use]
    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    /// Creates a `500 Internal Server Error` response.
    ///
    /// Returns a generic, non-sensitive message. The original error should be
    /// logged server-side before calling this method.
    #[must_use]
    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_server_error",
            "An internal server error occurred",
        )
    }

    /// Builds an HTTP response with the given request ID.
    ///
    /// This method allows handlers to provide the correlation ID from the
    /// request context so success and error responses share the same ID.
    #[must_use]
    pub fn into_response(
        self,
        clock: &(impl mockable::Clock + ?Sized),
        request_id: impl Into<String>,
    ) -> HttpResponse {
        self.with_request_id(request_id).response_with_clock(clock)
    }

    #[must_use]
    fn response_with_clock(&self, clock: &(impl mockable::Clock + ?Sized)) -> HttpResponse {
        json_error(
            clock,
            self.status,
            ErrorPayload::new(self.code, self.message.clone()),
            self.request_id
                .clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
        )
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        // Fallback for Actix's ResponseError integration when no injected
        // clock is available from the handler.
        self.response_with_clock(&DefaultClock)
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
            ConversationServiceError::ConversationNotFound(conversation_id) => {
                Self::not_found("conversation_not_found", conversation_id.to_string())
            }
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
