//! Repository port for task persistence, lookup, and association management.

use crate::task::domain::{BranchRef, IssueRef, PullRequestRef, Task, TaskId};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;

/// Result type for task repository operations.
pub type TaskRepositoryResult<T> = Result<T, TaskRepositoryError>;

/// Task persistence contract.
#[async_trait]
pub trait TaskRepository: Send + Sync {
    /// Stores a new task.
    ///
    /// # Errors
    ///
    /// Returns [`TaskRepositoryError::DuplicateTask`] when the task ID already
    /// exists or [`TaskRepositoryError::DuplicateIssueOrigin`] when the issue
    /// reference already maps to a task.
    async fn store(&self, task: &Task) -> TaskRepositoryResult<()>;

    /// Persists changes to an existing task (branch/PR associations, state,
    /// timestamps).
    ///
    /// # Errors
    ///
    /// Returns [`TaskRepositoryError::NotFound`] when the task does not exist.
    async fn update(&self, task: &Task) -> TaskRepositoryResult<()>;

    /// Finds a task by internal task identifier.
    ///
    /// Returns `None` when the task does not exist.
    async fn find_by_id(&self, id: TaskId) -> TaskRepositoryResult<Option<Task>>;

    /// Finds a task created from the given issue reference.
    ///
    /// Returns `None` when no task is associated with the issue reference.
    async fn find_by_issue_ref(&self, issue_ref: &IssueRef) -> TaskRepositoryResult<Option<Task>>;

    /// Returns all tasks linked to the given branch reference.
    ///
    /// Multiple tasks may share a branch (many-to-many relationship).
    async fn find_by_branch_ref(&self, branch_ref: &BranchRef) -> TaskRepositoryResult<Vec<Task>>;

    /// Returns all tasks linked to the given pull request reference.
    ///
    /// Multiple tasks may share a pull request (many-to-many relationship).
    async fn find_by_pull_request_ref(
        &self,
        pr_ref: &PullRequestRef,
    ) -> TaskRepositoryResult<Vec<Task>>;
}

/// Errors returned by task repository implementations.
#[derive(Debug, Clone, Error)]
pub enum TaskRepositoryError {
    /// A task with the same identifier already exists.
    #[error("duplicate task identifier: {0}")]
    DuplicateTask(TaskId),

    /// A task for the issue reference already exists.
    #[error("duplicate issue origin: {0}")]
    DuplicateIssueOrigin(IssueRef),

    /// The task was not found.
    #[error("task not found: {0}")]
    NotFound(TaskId),

    /// Persistence-layer failure.
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}

impl TaskRepositoryError {
    /// Wraps a persistence error.
    pub fn persistence(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Persistence(Arc::new(err))
    }
}
