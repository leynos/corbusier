//! Then steps for task state transition BDD scenarios.

use super::world::TaskTransitionWorld;
use corbusier::task::{
    domain::{TaskDomainError, TaskState},
    services::TaskLifecycleError,
};
use rstest_bdd_macros::then;

#[then(r#"the task state is "{state}""#)]
fn task_state_is(world: &TaskTransitionWorld, state: String) -> Result<(), eyre::Report> {
    let expected_state = TaskState::try_from(state.as_str())
        .map_err(|err| eyre::eyre!("invalid expected state in scenario: {err}"))?;

    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task"))?;

    if task.state() != expected_state {
        return Err(eyre::eyre!(
            "expected state {}, found {}",
            expected_state.as_str(),
            task.state().as_str()
        ));
    }

    Ok(())
}

#[then("the transition fails with an invalid state transition error")]
fn transition_fails_with_invalid_state_transition(
    world: &TaskTransitionWorld,
) -> Result<(), eyre::Report> {
    let result = world
        .last_transition_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing transition result"))?;

    if !matches!(
        result,
        Err(TaskLifecycleError::Domain(
            TaskDomainError::InvalidStateTransition { .. }
        ))
    ) {
        return Err(eyre::eyre!(
            "expected InvalidStateTransition error, got {result:?}"
        ));
    }

    Ok(())
}

#[then("the transition fails with an invalid state error")]
fn transition_fails_with_invalid_state_error(
    world: &TaskTransitionWorld,
) -> Result<(), eyre::Report> {
    let result = world
        .last_transition_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing transition result"))?;

    if !matches!(result, Err(TaskLifecycleError::InvalidState(_))) {
        return Err(eyre::eyre!("expected InvalidState error, got {result:?}"));
    }

    Ok(())
}
