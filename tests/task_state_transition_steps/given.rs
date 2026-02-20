//! Given steps for task state transition BDD scenarios.

use super::world::{TaskTransitionWorld, run_async};
use corbusier::task::services::{CreateTaskFromIssueRequest, TransitionTaskRequest};
use eyre::WrapErr;
use rstest_bdd_macros::given;

#[given(r#"an external issue "{provider}" "{repository}" #{issue_number:u64}"#)]
fn external_issue(
    world: &mut TaskTransitionWorld,
    provider: String,
    repository: String,
    issue_number: u64,
) {
    world.pending_issue_ref = Some((provider, repository, issue_number));
}

#[given(r#"the issue has title "{title}""#)]
fn issue_has_title(world: &mut TaskTransitionWorld, title: String) -> Result<(), eyre::Report> {
    let (provider, repository, issue_number) = world
        .pending_issue_ref
        .clone()
        .ok_or_else(|| eyre::eyre!("missing pending issue reference in scenario world"))?;
    world.pending_request = Some(CreateTaskFromIssueRequest::new(
        provider,
        repository,
        issue_number,
        title,
    ));
    Ok(())
}

#[given("the issue has been converted into a task")]
fn issue_converted_to_task(world: &mut TaskTransitionWorld) -> Result<(), eyre::Report> {
    let request = world
        .pending_request
        .clone()
        .ok_or_else(|| eyre::eyre!("missing pending request in scenario world"))?;
    let created = run_async(world.service.create_from_issue(request))
        .wrap_err("create task from issue for transition scenario")?;
    world.last_created_task = Some(created);
    Ok(())
}

#[given(r#"the task has been transitioned to "{target_state}""#)]
fn task_has_been_transitioned(
    world: &mut TaskTransitionWorld,
    target_state: String,
) -> Result<(), eyre::Report> {
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task in scenario world"))?;

    let transitioned = run_async(
        world
            .service
            .transition_task(TransitionTaskRequest::new(task.id(), target_state)),
    )
    .wrap_err("transition task in scenario setup")?;

    world.last_created_task = Some(transitioned);
    Ok(())
}
