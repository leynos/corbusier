//! When steps for branch and pull request association BDD scenarios.

use super::world::{TaskBranchPrWorld, run_async};
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use corbusier::task::services::{AssociateBranchRequest, AssociatePullRequestRequest};
use rstest_bdd_macros::when;

#[when(r#"a branch "{provider}" "{repository}" "{branch_name}" is associated with the task"#)]
fn associate_branch(
    world: &mut TaskBranchPrWorld,
    provider: String,
    repository: String,
    branch_name: String,
) -> Result<(), eyre::Report> {
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task in scenario world"))?;
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );
    let request = AssociateBranchRequest::new(task.id(), provider, repository, branch_name);
    let result = run_async(world.service.associate_branch(&ctx, request));
    if let Ok(ref updated) = result {
        world.last_created_task = Some(updated.clone());
    }
    world.last_associate_branch_result = Some(result);
    Ok(())
}

#[when(
    r#"a pull request "{provider}" "{repository}" #{pr_number:u64} is associated with the task"#
)]
fn associate_pull_request(
    world: &mut TaskBranchPrWorld,
    provider: String,
    repository: String,
    pr_number: u64,
) -> Result<(), eyre::Report> {
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task in scenario world"))?;
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );
    let request = AssociatePullRequestRequest::new(task.id(), provider, repository, pr_number);
    let result = run_async(world.service.associate_pull_request(&ctx, request));
    if let Ok(ref updated) = result {
        world.last_created_task = Some(updated.clone());
    }
    world.last_associate_pr_result = Some(result);
    Ok(())
}

#[when("a second branch is associated with the task")]
fn associate_second_branch(world: &mut TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task in scenario world"))?;
    let request = AssociateBranchRequest::new(
        task.id(),
        "github",
        "corbusier/core",
        "feature/second-branch",
    );
    let result = run_async(world.service.associate_branch(&ctx, request));
    world.last_associate_branch_result = Some(result);
    Ok(())
}

#[when("a second pull request is associated with the task")]
fn associate_second_pr(world: &mut TaskBranchPrWorld) -> Result<(), eyre::Report> {
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );
    let task = world
        .last_created_task
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing created task in scenario world"))?;
    let request = AssociatePullRequestRequest::new(task.id(), "github", "corbusier/core", 200);
    let result = run_async(world.service.associate_pull_request(&ctx, request));
    world.last_associate_pr_result = Some(result);
    Ok(())
}
