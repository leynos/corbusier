//! In-memory repository for task lifecycle tests.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::task::{
    domain::{IssueRef, Task, TaskId},
    ports::{TaskRepository, TaskRepositoryError, TaskRepositoryResult},
};

/// Thread-safe in-memory task repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryTaskRepository {
    tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    issue_index: Arc<RwLock<HashMap<IssueRef, TaskId>>>,
}

impl InMemoryTaskRepository {
    /// Creates an empty in-memory repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl TaskRepository for InMemoryTaskRepository {
    async fn store(&self, task: &Task) -> TaskRepositoryResult<()> {
        let mut tasks = self.tasks.write().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        if tasks.contains_key(&task.id()) {
            return Err(TaskRepositoryError::DuplicateTask(task.id()));
        }

        let mut issue_index = self.issue_index.write().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        let issue_ref = task.origin().issue_ref().clone();
        if issue_index.contains_key(&issue_ref) {
            return Err(TaskRepositoryError::DuplicateIssueOrigin(issue_ref));
        }

        issue_index.insert(issue_ref, task.id());
        tasks.insert(task.id(), task.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: TaskId) -> TaskRepositoryResult<Option<Task>> {
        let tasks = self.tasks.read().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(tasks.get(&id).cloned())
    }

    async fn find_by_issue_ref(&self, issue_ref: &IssueRef) -> TaskRepositoryResult<Option<Task>> {
        let maybe_task_id = self
            .issue_index
            .read()
            .map_err(|err| {
                TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
            })?
            .get(issue_ref)
            .copied();
        let Some(task_id) = maybe_task_id else {
            return Ok(None);
        };

        self.find_by_id(task_id).await
    }
}
