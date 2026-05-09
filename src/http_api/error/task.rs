//! Task HTTP error mappings.

use super::ApiError;
use crate::task::{domain::TaskDomainError, ports::TaskRepositoryError};
use serde_json::json;

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
            ApiError::conflict("branch_already_associated", error.to_string())
                .with_details(json!({ "taskId": task_id }))
        }
        TaskDomainError::PullRequestAlreadyAssociated(task_id) => {
            ApiError::conflict("pull_request_already_associated", error.to_string())
                .with_details(json!({ "taskId": task_id }))
        }
        TaskDomainError::InvalidStateTransition { task_id, from, to } => {
            ApiError::conflict("invalid_task_transition", error.to_string()).with_details(json!({
                "taskId": task_id,
                "from": from,
                "to": to,
            }))
        }
    }
}

pub(crate) fn map_task_repository_error(error: TaskRepositoryError) -> ApiError {
    match error {
        TaskRepositoryError::DuplicateTask(task_id) => {
            ApiError::conflict("duplicate_task", format!("task {task_id} already exists"))
                .with_details(json!({ "taskId": task_id }))
        }
        TaskRepositoryError::DuplicateIssueOrigin(issue_ref) => ApiError::conflict(
            "duplicate_issue_origin",
            format!("task already exists for issue {issue_ref}"),
        )
        .with_details(json!({ "issueRef": issue_ref.to_string() })),
        TaskRepositoryError::NotFound(task_id) => {
            ApiError::not_found("task_not_found", format!("task {task_id} was not found"))
                .with_details(json!({ "taskId": task_id }))
        }
        TaskRepositoryError::Persistence(err) => {
            tracing::error!(error = %err, "task persistence error");
            ApiError::internal()
        }
    }
}
