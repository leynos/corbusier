//! In-memory repository for task lifecycle tests.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::context::{RequestContext, TenantId};
use crate::task::{
    domain::{BranchRef, IssueRef, PullRequestRef, Task, TaskId},
    ports::{TaskRepository, TaskRepositoryError, TaskRepositoryResult},
};

/// Thread-safe in-memory task repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryTaskRepository {
    state: Arc<RwLock<HashMap<TenantId, TenantTaskState>>>,
}

#[derive(Debug, Default)]
struct TenantTaskState {
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

    /// Acquires a read lock, converting poison errors to repository errors.
    fn read_state(
        &self,
    ) -> TaskRepositoryResult<RwLockReadGuard<'_, HashMap<TenantId, TenantTaskState>>> {
        self.state
            .read()
            .map_err(|err| TaskRepositoryError::persistence(std::io::Error::other(err.to_string())))
    }

    /// Acquires a write lock, converting poison errors to repository errors.
    fn write_state(
        &self,
    ) -> TaskRepositoryResult<RwLockWriteGuard<'_, HashMap<TenantId, TenantTaskState>>> {
        self.state
            .write()
            .map_err(|err| TaskRepositoryError::persistence(std::io::Error::other(err.to_string())))
    }
}

fn index_issue(state: &mut TenantTaskState, task: &Task) {
    let issue_ref = task.origin().issue_ref().clone();
    state.issue_index.insert(issue_ref, task.id());
}

fn index_branch(state: &mut TenantTaskState, task: &Task) {
    if let Some(branch_ref) = task.branch_ref() {
        let key = branch_ref.to_string();
        state.branch_index.entry(key).or_default().push(task.id());
    }
}

fn index_pull_request(state: &mut TenantTaskState, task: &Task) {
    if let Some(pr_ref) = task.pull_request_ref() {
        let key = pr_ref.to_string();
        state
            .pull_request_index
            .entry(key)
            .or_default()
            .push(task.id());
    }
}

/// Removes a task ID from a string-keyed index, cleaning up the entry if empty.
fn remove_from_index(index: &mut HashMap<String, Vec<TaskId>>, task_id: TaskId, key: &str) {
    if let Some(ids) = index.get_mut(key) {
        ids.retain(|id| *id != task_id);
        if ids.is_empty() {
            index.remove(key);
        }
    }
}

/// Helper to look up tasks by index key.
fn find_by_index(
    state: &TenantTaskState,
    index: &HashMap<String, Vec<TaskId>>,
    key: &str,
) -> Vec<Task> {
    index
        .get(key)
        .map(|ids| {
            ids.iter()
                .filter_map(|id| state.tasks.get(id).cloned())
                .collect()
        })
        .unwrap_or_default()
}

#[async_trait]
impl TaskRepository for InMemoryTaskRepository {
    async fn store(&self, ctx: &RequestContext, task: &Task) -> TaskRepositoryResult<()> {
        let mut tenants = self.write_state()?;
        let state = tenants.entry(ctx.tenant_id()).or_default();

        if state.tasks.contains_key(&task.id()) {
            return Err(TaskRepositoryError::DuplicateTask(task.id()));
        }

        let issue_ref = task.origin().issue_ref().clone();
        if state.issue_index.contains_key(&issue_ref) {
            return Err(TaskRepositoryError::DuplicateIssueOrigin(issue_ref));
        }

        index_issue(state, task);
        index_branch(state, task);
        index_pull_request(state, task);
        state.tasks.insert(task.id(), task.clone());
        Ok(())
    }

    async fn update(&self, ctx: &RequestContext, task: &Task) -> TaskRepositoryResult<()> {
        let mut tenants = self.write_state()?;
        let state = tenants
            .get_mut(&ctx.tenant_id())
            .ok_or(TaskRepositoryError::NotFound(task.id()))?;

        let old_task = state
            .tasks
            .get(&task.id())
            .ok_or(TaskRepositoryError::NotFound(task.id()))?
            .clone();

        // Remove old issue/branch/PR index entries before adding updated ones.
        let old_issue = old_task.origin().issue_ref().clone();
        state.issue_index.remove(&old_issue);

        if let Some(old_branch) = old_task.branch_ref() {
            remove_from_index(&mut state.branch_index, task.id(), &old_branch.to_string());
        }
        if let Some(old_pr) = old_task.pull_request_ref() {
            remove_from_index(
                &mut state.pull_request_index,
                task.id(),
                &old_pr.to_string(),
            );
        }

        index_issue(state, task);
        index_branch(state, task);
        index_pull_request(state, task);
        state.tasks.insert(task.id(), task.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        ctx: &RequestContext,
        id: TaskId,
    ) -> TaskRepositoryResult<Option<Task>> {
        let tenants = self.read_state()?;
        Ok(tenants
            .get(&ctx.tenant_id())
            .and_then(|state| state.tasks.get(&id).cloned()))
    }

    async fn find_by_issue_ref(
        &self,
        ctx: &RequestContext,
        issue_ref: &IssueRef,
    ) -> TaskRepositoryResult<Option<Task>> {
        let tenants = self.read_state()?;
        let task = tenants.get(&ctx.tenant_id()).and_then(|state| {
            state
                .issue_index
                .get(issue_ref)
                .and_then(|task_id| state.tasks.get(task_id))
                .cloned()
        });
        Ok(task)
    }

    async fn find_by_branch_ref(
        &self,
        ctx: &RequestContext,
        branch_ref: &BranchRef,
    ) -> TaskRepositoryResult<Vec<Task>> {
        let tenants = self.read_state()?;
        let key = branch_ref.to_string();
        Ok(tenants
            .get(&ctx.tenant_id())
            .map(|state| find_by_index(state, &state.branch_index, &key))
            .unwrap_or_default())
    }

    async fn find_by_pull_request_ref(
        &self,
        ctx: &RequestContext,
        pr_ref: &PullRequestRef,
    ) -> TaskRepositoryResult<Vec<Task>> {
        let tenants = self.read_state()?;
        let key = pr_ref.to_string();
        Ok(tenants
            .get(&ctx.tenant_id())
            .map(|state| find_by_index(state, &state.pull_request_index, &key))
            .unwrap_or_default())
    }
}
