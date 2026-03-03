//! Behaviour tests for hook engine execution.

mod hook_engine_execution_steps;

use hook_engine_execution_steps::world::{HookWorld, world};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "tests/features/hook_engine_execution.feature",
    name = "Pre-commit hook executes successfully"
)]
#[tokio::test(flavor = "multi_thread")]
async fn pre_commit_hook_executes(world: HookWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/hook_engine_execution.feature",
    name = "Post-deploy hook failure is recorded"
)]
#[tokio::test(flavor = "multi_thread")]
async fn post_deploy_hook_failure(world: HookWorld) {
    let _ = world;
}
