//! Behaviour tests for issue-to-task creation and tracking.

mod task_issue_steps;

use rstest_bdd_macros::scenario;
use task_issue_steps::world::{TaskWorld, world};

#[scenario(
    path = "tests/features/task_issue_creation.feature",
    name = "Create task from issue and retrieve by reference"
)]
#[tokio::test(flavor = "multi_thread")]
async fn create_and_retrieve_task(world: TaskWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/task_issue_creation.feature",
    name = "Reject duplicate task creation for the same issue"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_duplicate_issue_mapping(world: TaskWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/task_issue_creation.feature",
    name = "Return no task for an unknown issue reference"
)]
#[tokio::test(flavor = "multi_thread")]
async fn missing_issue_lookup_returns_none(world: TaskWorld) {
    let _ = world;
}
