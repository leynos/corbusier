//! Given steps for branch and pull request association BDD scenarios.

use super::world::{TaskBranchPrWorld, run_async};
use corbusier::task::services::{
    AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
};
use eyre::WrapErr;
use rstest_bdd_macros::given;

#[given(r#"an external issue "{provider}" "{repository}" #{issue_number:u64}"#)]
fn external_issue(
    world: &mut TaskBranchPrWorld,
    provider: String,
    repository: String,
    issue_number: u64,
) {
    world.pending_issue_ref = Some((provider, repository, issue_number));
}

#[given(r#"the issue has title "{title}""#)]
fn issue_has_title(world: &mut TaskBranchPrWorld, title: String) -> Result<(), eyre::Report> {
    let (provider, repository, issue_number) = world
        .pending_issue_ref
        .clone()
        .ok_or_else(|| eyre::eyre!("missing pending issue reference in scenario world"))?;
    world.pending_issue_title = Some(title.clone());
    world.pending_request = Some(CreateTaskFromIssueRequest::new(
        provider,
        repository,
        issue_number,
        title,
    ));
    Ok(())
}

#[given("the issue has been converted into a task")]
fn issue_converted_to_task(world: &mut TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let request = world
        .pending_request
        .clone()
        .ok_or_else(|| eyre::eyre!("missing pending request in scenario world"))?;
    let created = run_async(world.service.create_from_issue(request))
        .wrap_err("create task from issue for association scenario")?;
    world.last_created_task = Some(created);
    Ok(())
}

#[given("a branch is already associated with the task")]
fn branch_already_associated(world: &mut TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task in scenario world"))?;
    let request = AssociateBranchRequest::new(
        task.id(),
        "github",
        "corbusier/core",
        "feature/initial-branch",
    );
    let updated = run_async(world.service.associate_branch(request))
        .wrap_err("associate initial branch for duplicate scenario")?;
    world.last_created_task = Some(updated);
    Ok(())
}

#[given("a pull request is already associated with the task")]
fn pr_already_associated(world: &mut TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task in scenario world"))?;
    let request = AssociatePullRequestRequest::new(task.id(), "github", "corbusier/core", 100);
    let updated = run_async(world.service.associate_pull_request(request))
        .wrap_err("associate initial PR for duplicate scenario")?;
    world.last_created_task = Some(updated);
    Ok(())
}
