//! Service layer for task lifecycle orchestration.
//!
//! Provides [`TaskLifecycleService`] which coordinates issue-to-task creation,
//! branch and pull request association, and lookup operations.

use crate::task::{
    domain::{
        BranchRef, ExternalIssue, ExternalIssueMetadata, IssueRef, PullRequestRef, Task,
        TaskDomainError, TaskId,
    },
    ports::{TaskRepository, TaskRepositoryError},
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

/// Request payload for associating a branch with an existing task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociateBranchRequest {
    task_id: TaskId,
    provider: String,
    repository: String,
    branch_name: String,
}

impl AssociateBranchRequest {
    /// Creates a branch association request.
    #[must_use]
    pub fn new(
        task_id: TaskId,
        provider: impl Into<String>,
        repository: impl Into<String>,
        branch_name: impl Into<String>,
    ) -> Self {
        Self {
            task_id,
            provider: provider.into(),
            repository: repository.into(),
            branch_name: branch_name.into(),
        }
    }
}

/// Request payload for associating a pull request with an existing task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociatePullRequestRequest {
    task_id: TaskId,
    provider: String,
    repository: String,
    pull_request_number: u64,
}

impl AssociatePullRequestRequest {
    /// Creates a pull request association request.
    #[must_use]
    pub fn new(
        task_id: TaskId,
        provider: impl Into<String>,
        repository: impl Into<String>,
        pull_request_number: u64,
    ) -> Self {
        Self {
            task_id,
            provider: provider.into(),
            repository: repository.into(),
            pull_request_number,
        }
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
    fn apply_optional_metadata(
        metadata: ExternalIssueMetadata,
        description: Option<String>,
        milestone: Option<String>,
    ) -> ExternalIssueMetadata {
        let metadata_with_description = match description {
            Some(value) => metadata.with_description(value),
            None => metadata,
        };

        match milestone {
            Some(value) => metadata_with_description.with_milestone(value),
            None => metadata_with_description,
        }
    }

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
        let CreateTaskFromIssueRequest {
            provider,
            repository,
            issue_number,
            title,
            description,
            labels,
            assignees,
            milestone,
        } = request;

        let issue_ref = IssueRef::from_parts(&provider, &repository, issue_number)?;
        let base_metadata = ExternalIssueMetadata::new(title)?
            .with_labels(labels)
            .with_assignees(assignees);
        let metadata = Self::apply_optional_metadata(base_metadata, description, milestone);

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
        Ok(self.repository.find_by_issue_ref(issue_ref).await?)
    }

    /// Associates a branch with an existing task.
    ///
    /// # Errors
    ///
    /// Returns [`TaskLifecycleError::Domain`] when input validation or the
    /// association invariant fails, or [`TaskLifecycleError::Repository`]
    /// when the task is not found or persistence fails.
    pub async fn associate_branch(
        &self,
        request: AssociateBranchRequest,
    ) -> TaskLifecycleResult<Task> {
        let AssociateBranchRequest {
            task_id,
            provider,
            repository,
            branch_name,
        } = request;

        let branch_ref = BranchRef::from_parts(&provider, &repository, &branch_name)?;

        let mut task = self
            .repository
            .find_by_id(task_id)
            .await?
            .ok_or(TaskRepositoryError::NotFound(task_id))?;

        task.associate_branch(branch_ref, &*self.clock)?;
        self.repository.update(&task).await?;
        Ok(task)
    }

    /// Associates a pull request with an existing task and transitions the
    /// task state to `InReview`.
    ///
    /// # Errors
    ///
    /// Returns [`TaskLifecycleError::Domain`] when input validation or the
    /// association invariant fails, or [`TaskLifecycleError::Repository`]
    /// when the task is not found or persistence fails.
    pub async fn associate_pull_request(
        &self,
        request: AssociatePullRequestRequest,
    ) -> TaskLifecycleResult<Task> {
        let AssociatePullRequestRequest {
            task_id,
            provider,
            repository,
            pull_request_number,
        } = request;

        let pr_ref = PullRequestRef::from_parts(&provider, &repository, pull_request_number)?;

        let mut task = self
            .repository
            .find_by_id(task_id)
            .await?
            .ok_or(TaskRepositoryError::NotFound(task_id))?;

        task.associate_pull_request(pr_ref, &*self.clock)?;
        self.repository.update(&task).await?;
        Ok(task)
    }

    /// Retrieves all tasks linked to a branch reference.
    ///
    /// Multiple tasks may share a branch (many-to-many relationship).
    ///
    /// # Errors
    ///
    /// Returns [`TaskLifecycleError::Repository`] when persistence lookup
    /// fails.
    pub async fn find_by_branch_ref(
        &self,
        branch_ref: &BranchRef,
    ) -> TaskLifecycleResult<Vec<Task>> {
        Ok(self.repository.find_by_branch_ref(branch_ref).await?)
    }

    /// Retrieves all tasks linked to a pull request reference.
    ///
    /// Multiple tasks may share a pull request (many-to-many relationship).
    ///
    /// # Errors
    ///
    /// Returns [`TaskLifecycleError::Repository`] when persistence lookup
    /// fails.
    pub async fn find_by_pull_request_ref(
        &self,
        pr_ref: &PullRequestRef,
    ) -> TaskLifecycleResult<Vec<Task>> {
        Ok(self.repository.find_by_pull_request_ref(pr_ref).await?)
    }
}
