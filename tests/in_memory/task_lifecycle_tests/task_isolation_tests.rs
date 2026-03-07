//! Tests for cross-tenant task isolation.

use super::{TestService, assert_single_task_found, service};
use crate::in_memory::helpers::{ctx, other_ctx};
use corbusier::context::RequestContext;
use corbusier::task::{
    domain::{BranchRef, IssueRef, PullRequestRef, Task},
    ports::TaskRepositoryError,
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TaskLifecycleError, TransitionTaskRequest,
    },
};
use rstest::rstest;

const PROVIDER: &str = "github";
const REPO: &str = "corbusier/core";
const ISSUE_NO: u64 = 900;
const BRANCH: &str = "feature/cross-tenant";

async fn create_task(
    service: &TestService,
    ctx: &RequestContext,
    title: &str,
) -> Result<Task, TaskLifecycleError> {
    service
        .create_from_issue(
            ctx,
            CreateTaskFromIssueRequest::new(PROVIDER, REPO, ISSUE_NO, title),
        )
        .await
}

const fn is_repo_not_found(e: &TaskLifecycleError) -> bool {
    matches!(
        e,
        TaskLifecycleError::Repository(TaskRepositoryError::NotFound(_))
    )
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn issue_visibility_is_scoped_to_tenant(
    service: TestService,
    ctx: RequestContext,
) -> Result<(), eyre::Report> {
    create_task(&service, &ctx, "Tenant isolation test").await?;

    let issue_ref = IssueRef::from_parts(PROVIDER, REPO, ISSUE_NO)?;

    let found = service.find_by_issue_ref(&other_ctx(), &issue_ref).await?;

    assert!(
        found.is_none(),
        "other tenant must not see tasks via issue ref"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn transition_in_other_tenant_fails_with_not_found(
    service: TestService,
    ctx: RequestContext,
) -> Result<(), eyre::Report> {
    let task = create_task(&service, &ctx, "Tenant isolation test").await?;

    let result = service
        .transition_task(
            &other_ctx(),
            TransitionTaskRequest::new(task.id(), "in_progress"),
        )
        .await;

    let err = result.expect_err("transition under other tenant should fail");
    assert!(is_repo_not_found(&err), "expected NotFound, got {err}");
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn branch_lookup_is_scoped_to_tenant(
    service: TestService,
    ctx: RequestContext,
) -> Result<(), eyre::Report> {
    let task = create_task(&service, &ctx, "Tenant isolation test").await?;

    service
        .associate_branch(
            &ctx,
            AssociateBranchRequest::new(task.id(), PROVIDER, REPO, BRANCH),
        )
        .await?;

    let branch_ref = BranchRef::from_parts(PROVIDER, REPO, BRANCH)?;

    let found_own = service.find_by_branch_ref(&ctx, &branch_ref).await?;
    assert_single_task_found(&found_own, task.id())?;

    let found_other = service
        .find_by_branch_ref(&other_ctx(), &branch_ref)
        .await?;
    assert!(
        found_other.is_empty(),
        "other tenant must not see branch associations"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn pr_lookup_is_scoped_to_tenant(
    service: TestService,
    ctx: RequestContext,
) -> Result<(), eyre::Report> {
    let task = create_task(&service, &ctx, "Tenant isolation test").await?;

    // Branch must be associated before a PR can be linked.
    service
        .associate_branch(
            &ctx,
            AssociateBranchRequest::new(task.id(), PROVIDER, REPO, BRANCH),
        )
        .await?;

    service
        .associate_pull_request(
            &ctx,
            AssociatePullRequestRequest::new(task.id(), PROVIDER, REPO, ISSUE_NO),
        )
        .await?;

    let pr_ref = PullRequestRef::from_parts(PROVIDER, REPO, ISSUE_NO)?;

    let found_own = service.find_by_pull_request_ref(&ctx, &pr_ref).await?;
    assert_single_task_found(&found_own, task.id())?;

    let found_other = service
        .find_by_pull_request_ref(&other_ctx(), &pr_ref)
        .await?;
    assert!(
        found_other.is_empty(),
        "other tenant must not see PR associations"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_issue_across_tenants_is_allowed(
    service: TestService,
    ctx: RequestContext,
) -> Result<(), eyre::Report> {
    create_task(&service, &ctx, "Tenant isolation test").await?;
    create_task(&service, &other_ctx(), "Same issue different tenant").await?;
    Ok(())
}
