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

use super::response::{ErrorPayload, json_error};
use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use mockable::DefaultClock;
use std::fmt;
use uuid::Uuid;

use crate::message::{
    error::{RepositoryError, ValidationError},
    services::ConversationServiceError,
};
use crate::task::{
    domain::TaskDomainError, ports::TaskRepositoryError, services::TaskLifecycleError,
};
use crate::tool_registry::{
    domain::ToolRegistryDomainError,
    ports::{
        McpServerHostError, McpServerRegistryError, ToolCatalogError, ToolGovernanceError,
        ToolLogStoreError,
    },
    services::ToolDiscoveryRoutingServiceError,
};

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

#[derive(Debug)]
enum ToolServiceInfrastructureError {
    Registry(McpServerRegistryError),
    Host(McpServerHostError),
}

fn map_tool_service_error(error: ToolDiscoveryRoutingServiceError) -> ApiError {
    match error {
        ToolDiscoveryRoutingServiceError::Registry(registry_error) => {
            map_tool_service_infrastructure_error(ToolServiceInfrastructureError::Registry(
                registry_error,
            ))
        }
        ToolDiscoveryRoutingServiceError::Host(host_error) => {
            map_tool_service_infrastructure_error(ToolServiceInfrastructureError::Host(host_error))
        }
        other => map_tool_service_client_error(other),
    }
}

fn map_tool_service_client_error(error: ToolDiscoveryRoutingServiceError) -> ApiError {
    match error {
        ToolDiscoveryRoutingServiceError::Domain(domain_error) => {
            map_tool_domain_error(domain_error)
        }
        ToolDiscoveryRoutingServiceError::Catalog(catalog_error) => {
            map_tool_catalog_error(catalog_error)
        }
        ToolDiscoveryRoutingServiceError::Governance(governance_error) => {
            map_tool_governance_error(&governance_error)
        }
        ToolDiscoveryRoutingServiceError::LogStore(log_store_error) => {
            map_tool_log_store_error(&log_store_error)
        }
        ToolDiscoveryRoutingServiceError::NotFound(server_id) => {
            ApiError::not_found("mcp_server_not_found", server_id.to_string())
        }
        ToolDiscoveryRoutingServiceError::Registry(_)
        | ToolDiscoveryRoutingServiceError::Host(_) => {
            debug_assert!(
                false,
                "infrastructure errors should be handled before client error mapping"
            );
            ApiError::internal()
        }
    }
}

fn map_tool_service_infrastructure_error(error: ToolServiceInfrastructureError) -> ApiError {
    match error {
        ToolServiceInfrastructureError::Registry(registry_error) => {
            log_tool_registry_error(&registry_error);
            ApiError::internal()
        }
        ToolServiceInfrastructureError::Host(host_error) => {
            log_tool_host_error(&host_error);
            ApiError::internal()
        }
    }
}

fn log_tool_registry_error(error: &McpServerRegistryError) {
    tracing::error!(error = %error, "registry error");
}

fn log_tool_host_error(error: &McpServerHostError) {
    tracing::error!(error = %error, "host error");
}

fn map_conversation_repository_error(
    error: crate::message::ports::ConversationRepositoryError,
) -> ApiError {
    match error {
        crate::message::ports::ConversationRepositoryError::DuplicateConversation(id) => {
            ApiError::conflict("duplicate_conversation", id.to_string())
        }
        crate::message::ports::ConversationRepositoryError::Persistence(err) => {
            tracing::error!(error = %err, "conversation repository persistence error");
            ApiError::internal()
        }
    }
}

#[expect(
    clippy::cognitive_complexity,
    reason = "Simple match arms on error variants"
)]
fn map_message_repository_error(error: RepositoryError) -> ApiError {
    match error {
        RepositoryError::ConversationNotFound(conversation_id) => {
            ApiError::not_found("conversation_not_found", conversation_id.to_string())
        }
        RepositoryError::NotFound(message_id) => {
            ApiError::not_found("message_not_found", message_id.to_string())
        }
        RepositoryError::DuplicateMessage(message_id) => {
            ApiError::conflict("duplicate_message", message_id.to_string())
        }
        RepositoryError::DuplicateSequence {
            conversation_id,
            sequence,
        } => ApiError::conflict(
            "duplicate_sequence",
            format!("conversation {conversation_id} already has sequence {sequence}"),
        ),
        RepositoryError::Database(err) => {
            tracing::error!(error = %err, "message database error");
            ApiError::internal()
        }
        RepositoryError::Connection(err) => {
            tracing::error!(error = %err, "message connection error");
            ApiError::internal()
        }
        RepositoryError::Serialization(message) => {
            ApiError::bad_request("serialization_error", message)
        }
    }
}

fn map_task_domain_error(error: &TaskDomainError) -> ApiError {
    match error {
        TaskDomainError::InvalidIssueProvider(_)
        | TaskDomainError::InvalidRepository(_)
        | TaskDomainError::InvalidIssueNumber(_)
        | TaskDomainError::EmptyIssueTitle
        | TaskDomainError::InvalidBranchName(_)
        | TaskDomainError::InvalidPullRequestNumber(_)
        | TaskDomainError::InvalidBranchRefFormat(_)
        | TaskDomainError::InvalidPullRequestRefFormat(_)
        | TaskDomainError::CanonicalRefTooLong(_) => {
            ApiError::bad_request("task_validation_failed", error.to_string())
        }
        TaskDomainError::BranchAlreadyAssociated(task_id) => {
            ApiError::conflict("branch_already_associated", task_id.to_string())
        }
        TaskDomainError::PullRequestAlreadyAssociated(task_id) => {
            ApiError::conflict("pull_request_already_associated", task_id.to_string())
        }
        TaskDomainError::InvalidStateTransition { .. } => {
            ApiError::conflict("invalid_task_transition", error.to_string())
        }
    }
}

fn map_task_repository_error(error: TaskRepositoryError) -> ApiError {
    match error {
        TaskRepositoryError::DuplicateTask(task_id) => {
            ApiError::conflict("duplicate_task", task_id.to_string())
        }
        TaskRepositoryError::DuplicateIssueOrigin(issue_ref) => {
            ApiError::conflict("duplicate_issue_origin", issue_ref.to_string())
        }
        TaskRepositoryError::NotFound(task_id) => {
            ApiError::not_found("task_not_found", task_id.to_string())
        }
        TaskRepositoryError::Persistence(err) => {
            tracing::error!(error = %err, "task persistence error");
            ApiError::internal()
        }
    }
}

fn map_tool_domain_error(error: ToolRegistryDomainError) -> ApiError {
    match error {
        ToolRegistryDomainError::EmptyServerName
        | ToolRegistryDomainError::InvalidServerName(_)
        | ToolRegistryDomainError::ServerNameTooLong(_)
        | ToolRegistryDomainError::EmptyStdioCommand
        | ToolRegistryDomainError::EmptyWorkingDirectory
        | ToolRegistryDomainError::EmptyHttpSseBaseUrl
        | ToolRegistryDomainError::InvalidHttpSseBaseUrl(_)
        | ToolRegistryDomainError::EmptyToolName
        | ToolRegistryDomainError::EmptyToolDescription => {
            ApiError::bad_request("tool_request_failed", error.to_string())
        }
        ToolRegistryDomainError::InvalidLifecycleTransition { .. } => {
            ApiError::conflict("invalid_mcp_server_transition", error.to_string())
        }
        ToolRegistryDomainError::ToolQueryRequiresRunning { server_id, .. } => {
            ApiError::conflict("mcp_server_not_running", server_id.to_string())
        }
        ToolRegistryDomainError::ToolNotFound(tool_name) => {
            ApiError::not_found("tool_not_found", tool_name)
        }
        ToolRegistryDomainError::ToolUnavailable { tool_name, .. } => {
            ApiError::conflict("tool_unavailable", tool_name)
        }
        ToolRegistryDomainError::PolicyDenied { reason, .. } => {
            ApiError::new(StatusCode::FORBIDDEN, "tool_policy_denied", reason)
        }
        ToolRegistryDomainError::SchemaValidationFailed { reason, .. } => {
            ApiError::bad_request("tool_schema_validation_failed", reason)
        }
        ToolRegistryDomainError::AmbiguousToolName { tool_name, .. } => {
            ApiError::conflict("ambiguous_tool_name", tool_name)
        }
        ToolRegistryDomainError::ToolCallTimeout { tool_name, .. } => {
            ApiError::new(StatusCode::GATEWAY_TIMEOUT, "tool_call_timeout", tool_name)
        }
    }
}

fn map_tool_catalog_error(error: ToolCatalogError) -> ApiError {
    match error {
        ToolCatalogError::DuplicateEntry { tool_name, .. }
        | ToolCatalogError::DuplicateWithinBatch { tool_name, .. } => {
            ApiError::conflict("duplicate_tool_catalog_entry", tool_name)
        }
        ToolCatalogError::NotFound(name) => {
            ApiError::not_found("tool_catalog_entry_not_found", name)
        }
        ToolCatalogError::MixedServerBatch { reason } => {
            ApiError::bad_request("mixed_tool_server_batch", reason)
        }
        ToolCatalogError::InvalidPersistedData { .. } | ToolCatalogError::Persistence { .. } => {
            tracing::error!(error = %error, "tool catalog error");
            ApiError::internal()
        }
    }
}

fn map_tool_governance_error(error: &ToolGovernanceError) -> ApiError {
    tracing::error!(error = %error, "tool governance error");
    ApiError::internal()
}

fn map_tool_log_store_error(error: &ToolLogStoreError) -> ApiError {
    tracing::error!(error = %error, "tool log store error");
    ApiError::internal()
}
