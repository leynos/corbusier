//! In-memory repository for task lifecycle tests.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::task::{
    domain::{BranchRef, IssueRef, PullRequestRef, Task, TaskId},
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
    branch_index: HashMap<String, Vec<TaskId>>,
    pull_request_index: HashMap<String, Vec<TaskId>>,
}

impl InMemoryTaskRepository {
    /// Creates an empty in-memory repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

fn index_branch(state: &mut InMemoryTaskState, task: &Task) {
    if let Some(branch_ref) = task.branch_ref() {
        let key = branch_ref.to_string();
        state.branch_index.entry(key).or_default().push(task.id());
    }
}

fn index_pull_request(state: &mut InMemoryTaskState, task: &Task) {
    if let Some(pr_ref) = task.pull_request_ref() {
        let key = pr_ref.to_string();
        state
            .pull_request_index
            .entry(key)
            .or_default()
            .push(task.id());
    }
}

fn remove_branch_index(state: &mut InMemoryTaskState, task_id: TaskId, key: &str) {
    if let Some(ids) = state.branch_index.get_mut(key) {
        ids.retain(|id| *id != task_id);
        if ids.is_empty() {
            state.branch_index.remove(key);
        }
    }
}

fn remove_pr_index(state: &mut InMemoryTaskState, task_id: TaskId, key: &str) {
    if let Some(ids) = state.pull_request_index.get_mut(key) {
        ids.retain(|id| *id != task_id);
        if ids.is_empty() {
            state.pull_request_index.remove(key);
        }
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
        index_branch(&mut state, task);
        index_pull_request(&mut state, task);
        state.tasks.insert(task.id(), task.clone());
        Ok(())
    }

    async fn update(&self, task: &Task) -> TaskRepositoryResult<()> {
        let mut state = self.state.write().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;

        let old_task = state
            .tasks
            .get(&task.id())
            .ok_or(TaskRepositoryError::NotFound(task.id()))?
            .clone();

        // Remove old branch/PR index entries before adding updated ones.
        if let Some(old_branch) = old_task.branch_ref() {
            remove_branch_index(&mut state, task.id(), &old_branch.to_string());
        }
        if let Some(old_pr) = old_task.pull_request_ref() {
            remove_pr_index(&mut state, task.id(), &old_pr.to_string());
        }

        index_branch(&mut state, task);
        index_pull_request(&mut state, task);
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

    async fn find_by_branch_ref(&self, branch_ref: &BranchRef) -> TaskRepositoryResult<Vec<Task>> {
        let state = self.state.read().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        let key = branch_ref.to_string();
        let tasks = state
            .branch_index
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| state.tasks.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default();
        Ok(tasks)
    }

    async fn find_by_pull_request_ref(
        &self,
        pr_ref: &PullRequestRef,
    ) -> TaskRepositoryResult<Vec<Task>> {
        let state = self.state.read().map_err(|err| {
            TaskRepositoryError::persistence(std::io::Error::other(err.to_string()))
        })?;
        let key = pr_ref.to_string();
        let tasks = state
            .pull_request_index
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| state.tasks.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default();
        Ok(tasks)
    }
}
