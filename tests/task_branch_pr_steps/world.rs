//! Shared world state for branch and pull request association BDD scenarios.

use std::sync::Arc;

use corbusier::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::Task,
    services::{CreateTaskFromIssueRequest, TaskLifecycleError, TaskLifecycleService},
};
use mockable::DefaultClock;
use rstest::fixture;

/// Service type used by the BDD world.
pub type TestTaskService = TaskLifecycleService<InMemoryTaskRepository, DefaultClock>;

/// Scenario world for branch and pull request association behaviour tests.
pub struct TaskBranchPrWorld {
    pub service: TestTaskService,
    pub pending_issue_ref: Option<(String, String, u64)>,
    pub pending_issue_title: Option<String>,
    pub pending_request: Option<CreateTaskFromIssueRequest>,
    pub last_created_task: Option<Task>,
    pub last_associate_branch_result: Option<Result<Task, TaskLifecycleError>>,
    pub last_associate_pr_result: Option<Result<Task, TaskLifecycleError>>,
}

impl TaskBranchPrWorld {
    /// Creates a world with empty pending scenario state.
    #[must_use]
    pub fn new() -> Self {
        let service = TaskLifecycleService::new(
            Arc::new(InMemoryTaskRepository::new()),
            Arc::new(DefaultClock),
        );
        Self {
            service,
            pending_issue_ref: None,
            pending_issue_title: None,
            pending_request: None,
            last_created_task: None,
            last_associate_branch_result: None,
            last_associate_pr_result: None,
        }
    }
}

impl Default for TaskBranchPrWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture that creates a new scenario world.
#[fixture]
pub fn world() -> TaskBranchPrWorld {
    TaskBranchPrWorld::default()
}

/// Runs an async operation within sync step definitions.
pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
