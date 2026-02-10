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
    state: Arc<RwLock<InMemoryTaskState>>,
}

#[derive(Debug, Default)]
struct InMemoryTaskState {
    tasks: HashMap<TaskId, Task>,
    issue_index: HashMap<IssueRef, TaskId>,
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
        let mut state = self.state.write().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        if state.tasks.contains_key(&task.id()) {
            return Err(TaskRepositoryError::DuplicateTask(task.id()));
        }

        let issue_ref = task.origin().issue_ref().clone();
        if state.issue_index.contains_key(&issue_ref) {
            return Err(TaskRepositoryError::DuplicateIssueOrigin(issue_ref));
        }

        state.issue_index.insert(issue_ref, task.id());
        state.tasks.insert(task.id(), task.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: TaskId) -> TaskRepositoryResult<Option<Task>> {
        let state = self.state.read().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        Ok(state.tasks.get(&id).cloned())
    }

    async fn find_by_issue_ref(&self, issue_ref: &IssueRef) -> TaskRepositoryResult<Option<Task>> {
        let state = self.state.read().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        let task = state
            .issue_index
            .get(issue_ref)
            .and_then(|task_id| state.tasks.get(task_id))
            .cloned();
        Ok(task)
    }
}
