//! Repository port for task persistence and issue-reference lookup.

use crate::task::domain::{IssueRef, Task, TaskId};
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

    /// Finds a task by internal task identifier.
    ///
    /// Returns `None` when the task does not exist.
    async fn find_by_id(&self, id: TaskId) -> TaskRepositoryResult<Option<Task>>;

    /// Finds a task created from the given issue reference.
    ///
    /// Returns `None` when no task is associated with the issue reference.
    async fn find_by_issue_ref(&self, issue_ref: &IssueRef) -> TaskRepositoryResult<Option<Task>>;
}

/// Errors returned by task repository implementations.
#[derive(Debug, Clone, Error)]
pub enum TaskRepositoryError {
    /// A task with the same identifier already exists.
    #[error("duplicate task identifier: {0}")]
    DuplicateTask(TaskId),

    /// A task for the issue reference already exists.
    #[error("duplicate issue origin: {0:?}")]
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
