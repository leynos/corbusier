//! Task HTTP error mappings.

use super::ApiError;
use crate::task::{domain::TaskDomainError, ports::TaskRepositoryError};

pub(crate) fn map_task_domain_error(error: &TaskDomainError) -> ApiError {
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

pub(crate) fn map_task_repository_error(error: TaskRepositoryError) -> ApiError {
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
