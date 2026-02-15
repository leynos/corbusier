//! Then steps for branch and pull request association BDD scenarios.

use super::world::{TaskBranchPrWorld, run_async};
use corbusier::task::{
    domain::{TaskDomainError, TaskState},
    services::TaskLifecycleError,
};
use rstest_bdd_macros::then;

#[then("the task has an associated branch reference")]
fn task_has_branch_ref(world: &TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_associate_branch_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing branch association result"))?;
    let task = result
        .as_ref()
        .map_err(|err| eyre::eyre!("unexpected branch association failure: {err}"))?;
    if task.branch_ref().is_none() {
        return Err(eyre::eyre!("expected task to have a branch reference"));
    }
    Ok(())
}

#[then("the task can be retrieved by the branch reference")]
fn task_retrievable_by_branch_ref(world: &TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task"))?;
    let branch_ref = task
        .branch_ref()
        .ok_or_else(|| eyre::eyre!("task should have a branch reference"))?;
    let found = run_async(world.service.find_by_branch_ref(branch_ref))
        .map_err(|err| eyre::eyre!("branch ref lookup failed: {err}"))?;
    if found.is_empty() {
        return Err(eyre::eyre!("expected at least one task for branch ref"));
    }
    if !found.iter().any(|t| t.id() == task.id()) {
        return Err(eyre::eyre!("expected task ID in branch ref lookup results"));
    }
    Ok(())
}

#[then("the task has an associated pull request reference")]
fn task_has_pr_ref(world: &TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_associate_pr_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing PR association result"))?;
    let task = result
        .as_ref()
        .map_err(|err| eyre::eyre!("unexpected PR association failure: {err}"))?;
    if task.pull_request_ref().is_none() {
        return Err(eyre::eyre!(
            "expected task to have a pull request reference"
        ));
    }
    Ok(())
}

#[then("the task state is in_review")]
fn task_state_is_in_review(world: &TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task"))?;
    if task.state() != TaskState::InReview {
        return Err(eyre::eyre!(
            "expected state InReview, found {}",
            task.state().as_str()
        ));
    }
    Ok(())
}

#[then("branch association fails with a branch already associated error")]
fn branch_association_fails_duplicate(world: &TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_associate_branch_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing branch association result"))?;
    if !matches!(
        result,
        Err(TaskLifecycleError::Domain(
            TaskDomainError::BranchAlreadyAssociated(_)
        ))
    ) {
        return Err(eyre::eyre!(
            "expected BranchAlreadyAssociated error, got {result:?}"
        ));
    }
    Ok(())
}

#[then("pull request association fails with a PR already associated error")]
fn pr_association_fails_duplicate(world: &TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_associate_pr_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing PR association result"))?;
    if !matches!(
        result,
        Err(TaskLifecycleError::Domain(
            TaskDomainError::PullRequestAlreadyAssociated(_)
        ))
    ) {
        return Err(eyre::eyre!(
            "expected PullRequestAlreadyAssociated error, got {result:?}"
        ));
    }
    Ok(())
}
