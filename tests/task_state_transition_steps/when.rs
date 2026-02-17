//! When steps for task state transition BDD scenarios.

use super::world::{TaskTransitionWorld, run_async};
use corbusier::task::services::TransitionTaskRequest;
use rstest_bdd_macros::when;

#[when(r#"the task is transitioned to "{target_state}""#)]
fn transition_task(
    world: &mut TaskTransitionWorld,
    target_state: String,
) -> Result<(), eyre::Report> {
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task in scenario world"))?;

    let result = run_async(
        world
            .service
            .transition_task(TransitionTaskRequest::new(task.id(), target_state)),
    );
    if let Ok(ref updated) = result {
        world.last_created_task = Some(updated.clone());
    }
    world.last_transition_result = Some(result);
    Ok(())
}
