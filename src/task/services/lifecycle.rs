//! Service layer for issue-to-task creation and retrieval.

use crate::task::{
    domain::{ExternalIssue, ExternalIssueMetadata, IssueRef, Task, TaskDomainError},
    ports::{TaskRepository, TaskRepositoryError, TaskRepositoryResult},
};
use mockable::Clock;
use std::sync::Arc;
use thiserror::Error;

/// Request payload for creating a task from external issue data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTaskFromIssueRequest {
    provider: String,
    repository: String,
    issue_number: u64,
    title: String,
    description: Option<String>,
    labels: Vec<String>,
    assignees: Vec<String>,
    milestone: Option<String>,
}

impl CreateTaskFromIssueRequest {
    /// Creates a request with required issue fields.
    #[must_use]
    pub fn new(
        provider: impl Into<String>,
        repository: impl Into<String>,
        issue_number: u64,
        title: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            repository: repository.into(),
            issue_number,
            title: title.into(),
            description: None,
            labels: Vec::new(),
            assignees: Vec::new(),
            milestone: None,
        }
    }

    /// Sets issue description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets issue labels.
    #[must_use]
    pub fn with_labels(mut self, labels: impl IntoIterator<Item = String>) -> Self {
        self.labels = labels.into_iter().collect();
        self
    }

    /// Sets issue assignees.
    #[must_use]
    pub fn with_assignees(mut self, assignees: impl IntoIterator<Item = String>) -> Self {
        self.assignees = assignees.into_iter().collect();
        self
    }

    /// Sets issue milestone.
    #[must_use]
    pub fn with_milestone(mut self, milestone: impl Into<String>) -> Self {
        self.milestone = Some(milestone.into());
        self
    }
}

/// Service-level errors for task lifecycle operations.
#[derive(Debug, Error)]
pub enum TaskLifecycleError {
    /// Domain validation failed.
    #[error(transparent)]
    Domain(#[from] TaskDomainError),
    /// Repository operation failed.
    #[error(transparent)]
    Repository(#[from] TaskRepositoryError),
}

/// Result type for task lifecycle service operations.
pub type TaskLifecycleResult<T> = Result<T, TaskLifecycleError>;

/// Task lifecycle orchestration service.
#[derive(Clone)]
pub struct TaskLifecycleService<R, C>
where
    R: TaskRepository,
    C: Clock + Send + Sync,
{
    repository: Arc<R>,
    clock: Arc<C>,
}

impl<R, C> TaskLifecycleService<R, C>
where
    R: TaskRepository,
    C: Clock + Send + Sync,
{
    /// Creates a new task lifecycle service.
    #[must_use]
    pub const fn new(repository: Arc<R>, clock: Arc<C>) -> Self {
        Self { repository, clock }
    }

    /// Creates a new task from external issue metadata.
    ///
    /// # Errors
    ///
    /// Returns [`TaskLifecycleError`] when input validation fails or the
    /// repository rejects persistence.
    pub async fn create_from_issue(
        &self,
        request: CreateTaskFromIssueRequest,
    ) -> TaskLifecycleResult<Task> {
        let issue_ref =
            IssueRef::from_parts(&request.provider, &request.repository, request.issue_number)?;

        let mut metadata = ExternalIssueMetadata::new(request.title)?;
        if let Some(description) = request.description {
            metadata = metadata.with_description(description);
        }
        metadata = metadata
            .with_labels(request.labels)
            .with_assignees(request.assignees);
        if let Some(milestone) = request.milestone {
            metadata = metadata.with_milestone(milestone);
        }

        let external_issue = ExternalIssue::new(issue_ref, metadata);
        let task = Task::new_from_issue(&external_issue, &*self.clock);
        self.repository.store(&task).await?;
        Ok(task)
    }

    /// Retrieves a task by issue reference.
    ///
    /// Returns `Ok(None)` when no task has been created from the issue.
    ///
    /// # Errors
    ///
    /// Returns [`TaskLifecycleError::Repository`] when persistence lookup
    /// fails.
    pub async fn find_by_issue_ref(
        &self,
        issue_ref: &IssueRef,
    ) -> TaskLifecycleResult<Option<Task>> {
        let result: TaskRepositoryResult<Option<Task>> =
            self.repository.find_by_issue_ref(issue_ref).await;
        Ok(result?)
    }
}
