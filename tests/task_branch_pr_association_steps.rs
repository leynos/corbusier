//! Behaviour tests for branch and pull request association with tasks.

mod task_branch_pr_steps;

use rstest_bdd_macros::scenario;
use task_branch_pr_steps::world::{TaskBranchPrWorld, world};

#[scenario(
    path = "tests/features/task_branch_pr_association.feature",
    name = "Associate a branch with a task and retrieve by reference"
)]
#[tokio::test(flavor = "multi_thread")]
async fn associate_branch_and_retrieve(world: TaskBranchPrWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/task_branch_pr_association.feature",
    name = "Associate a pull request with a task and verify state"
)]
#[tokio::test(flavor = "multi_thread")]
async fn associate_pr_and_verify_state(world: TaskBranchPrWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/task_branch_pr_association.feature",
    name = "Reject second branch association on the same task"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_duplicate_branch(world: TaskBranchPrWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/task_branch_pr_association.feature",
    name = "Reject second pull request association on the same task"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_duplicate_pr(world: TaskBranchPrWorld) {
    let _ = world;
}
