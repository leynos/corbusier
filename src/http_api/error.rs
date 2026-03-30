//! Shared HTTP error mapping.

use super::response::{ErrorPayload, json_error};
use actix_web::{HttpResponse, ResponseError, http::StatusCode};
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
    ports::{ToolCatalogError, ToolLogStoreError, ToolPolicyError},
    services::ToolDiscoveryRoutingServiceError,
};

/// Shared API error type.
#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    /// Creates a new API error.
    #[must_use]
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    /// Creates a `400 Bad Request` response.
    #[must_use]
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    /// Creates a `401 Unauthorized` response.
    #[must_use]
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
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
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
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
        json_error(
            self.status,
            ErrorPayload::new(self.code, self.message.clone()),
            Uuid::new_v4().to_string(),
        )
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
        match error {
            ToolDiscoveryRoutingServiceError::Domain(domain_error) => {
                map_tool_domain_error(domain_error)
            }
            ToolDiscoveryRoutingServiceError::Catalog(catalog_error) => {
                map_tool_catalog_error(catalog_error)
            }
            ToolDiscoveryRoutingServiceError::Registry(registry_error) => {
                Self::internal(registry_error.to_string())
            }
            ToolDiscoveryRoutingServiceError::Host(host_error) => {
                Self::internal(host_error.to_string())
            }
            ToolDiscoveryRoutingServiceError::Policy(policy_error) => {
                map_tool_policy_error(&policy_error)
            }
            ToolDiscoveryRoutingServiceError::LogStore(log_store_error) => {
                map_tool_log_store_error(&log_store_error)
            }
            ToolDiscoveryRoutingServiceError::NotFound(server_id) => {
                Self::not_found("mcp_server_not_found", server_id.to_string())
            }
        }
    }
}

fn map_conversation_repository_error(
    error: crate::message::ports::ConversationRepositoryError,
) -> ApiError {
    match error {
        crate::message::ports::ConversationRepositoryError::DuplicateConversation(id) => {
            ApiError::conflict("duplicate_conversation", id.to_string())
        }
        crate::message::ports::ConversationRepositoryError::Persistence(err) => {
            ApiError::internal(err.to_string())
        }
    }
}

fn map_message_repository_error(error: RepositoryError) -> ApiError {
    match error {
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
        RepositoryError::Database(err) => ApiError::internal(err.to_string()),
        RepositoryError::Connection(err) => ApiError::internal(err),
        RepositoryError::Serialization(message) => {
            ApiError::bad_request("serialization_error", message)
        }
    }
}

fn map_task_domain_error(error: &TaskDomainError) -> ApiError {
    match error {
        TaskDomainError::BranchAlreadyAssociated(task_id) => {
            ApiError::conflict("branch_already_associated", task_id.to_string())
        }
        TaskDomainError::PullRequestAlreadyAssociated(task_id) => {
            ApiError::conflict("pull_request_already_associated", task_id.to_string())
        }
        TaskDomainError::InvalidStateTransition { .. } => {
            ApiError::conflict("invalid_task_transition", error.to_string())
        }
        _ => ApiError::bad_request("task_validation_failed", error.to_string()),
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
        TaskRepositoryError::Persistence(err) => ApiError::internal(err.to_string()),
    }
}

fn map_tool_domain_error(error: ToolRegistryDomainError) -> ApiError {
    match error {
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
        _ => ApiError::bad_request("tool_request_failed", error.to_string()),
    }
}

fn map_tool_catalog_error(error: ToolCatalogError) -> ApiError {
    match error {
        ToolCatalogError::DuplicateEntry { tool_name, .. }
        | ToolCatalogError::DuplicateWithinBatch { tool_name, .. } => {
            ApiError::conflict("duplicate_tool_catalog_entry", tool_name)
        }
        _ => ApiError::internal(error.to_string()),
    }
}

fn map_tool_policy_error(error: &ToolPolicyError) -> ApiError {
    ApiError::internal(error.to_string())
}

fn map_tool_log_store_error(error: &ToolLogStoreError) -> ApiError {
    ApiError::internal(error.to_string())
}
