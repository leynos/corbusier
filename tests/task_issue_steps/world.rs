//! Shared world state for task lifecycle BDD scenarios.

use std::sync::Arc;

use corbusier::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::{IssueRef, Task},
    services::{CreateTaskFromIssueRequest, TaskLifecycleError, TaskLifecycleService},
};
use mockable::DefaultClock;
use rstest::fixture;

/// Service type used by the BDD world.
pub type TestTaskService = TaskLifecycleService<InMemoryTaskRepository, DefaultClock>;

/// Scenario world for task lifecycle behaviour tests.
pub struct TaskWorld {
    pub service: TestTaskService,
    pub pending_issue_ref: Option<(String, String, u64)>,
    pub pending_request: Option<CreateTaskFromIssueRequest>,
    pub pending_lookup: Option<IssueRef>,
    pub last_created_task: Option<Task>,
    pub last_create_result: Option<Result<Task, TaskLifecycleError>>,
    pub last_lookup_result: Option<Result<Option<Task>, TaskLifecycleError>>,
}

impl TaskWorld {
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
            pending_request: None,
            pending_lookup: None,
            last_created_task: None,
            last_create_result: None,
            last_lookup_result: None,
        }
    }
}

impl Default for TaskWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture that creates a new scenario world.
#[fixture]
pub fn world() -> TaskWorld {
    TaskWorld::default()
}

/// Runs an async operation within sync step definitions.
pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
