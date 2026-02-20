//! Behaviour tests for task state transition validation.

#[path = "task_state_transition_steps/mod.rs"]
mod task_state_transition_steps_defs;

use rstest_bdd_macros::scenario;
use task_state_transition_steps_defs::world::{TaskTransitionWorld, world};

#[scenario(
    path = "tests/features/task_state_transitions.feature",
    name = "Transition a draft task to in progress"
)]
#[tokio::test(flavor = "multi_thread")]
async fn transition_draft_to_in_progress(world: TaskTransitionWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/task_state_transitions.feature",
    name = "Reject transition from draft to done"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_draft_to_done(world: TaskTransitionWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/task_state_transitions.feature",
    name = "Reject transition from a terminal state"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_transition_from_terminal(world: TaskTransitionWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/task_state_transitions.feature",
    name = "Reject transition with an invalid state string"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_transition_with_invalid_state_string(world: TaskTransitionWorld) {
    let _ = world;
}
