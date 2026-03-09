//! Tests for cross-tenant task isolation.

use super::{TestService, assert_single_task_found, service};
use crate::in_memory::helpers::ctx;
use corbusier::context::{RequestContext, TenantId};
use corbusier::task::{
    domain::{BranchRef, IssueRef, PullRequestRef, Task},
    ports::TaskRepositoryError,
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TaskLifecycleError, TransitionTaskRequest,
    },
};
use rstest::rstest;

/// Creates a context that differs from `source` only in `tenant_id`.
///
/// This isolates exactly the tenant dimension so assertions prove
/// that tenant scoping — not user or session identity — drives
/// visibility.
fn ctx_other_tenant(source: &RequestContext) -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        source.correlation_id(),
        source.user_id(),
        source.session_id(),
    )
}

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
    let task = create_task(&service, &ctx, "Tenant isolation test").await?;

    let issue_ref = IssueRef::from_parts(PROVIDER, REPO, ISSUE_NO)?;

    // Positive control: owning tenant resolves its own task.
    let found_own = service.find_by_issue_ref(&ctx, &issue_ref).await?;
    let own_task = found_own.expect("owning tenant must find its task");
    assert_eq!(own_task.id(), task.id());

    // Cross-tenant: other tenant cannot see the task.
    let ctx_b = ctx_other_tenant(&ctx);
    let found_other = service.find_by_issue_ref(&ctx_b, &issue_ref).await?;
    assert!(
        found_other.is_none(),
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
    let initial_state = task.state();

    let ctx_b = ctx_other_tenant(&ctx);
    let result = service
        .transition_task(&ctx_b, TransitionTaskRequest::new(task.id(), "in_progress"))
        .await;

    let err = result.expect_err("transition under other tenant should fail");
    assert!(is_repo_not_found(&err), "expected NotFound, got {err}");

    // Verify the rejected transition had no side effects on the
    // owning tenant's task.
    let issue_ref = IssueRef::from_parts(PROVIDER, REPO, ISSUE_NO)?;
    let refetched = service
        .find_by_issue_ref(&ctx, &issue_ref)
        .await?
        .expect("owning tenant must still find its task");
    assert_eq!(
        refetched.state(),
        initial_state,
        "task state must be unchanged after rejected cross-tenant transition",
    );
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

    let ctx_b = ctx_other_tenant(&ctx);
    let found_other = service.find_by_branch_ref(&ctx_b, &branch_ref).await?;
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

    let ctx_b = ctx_other_tenant(&ctx);
    let found_other = service.find_by_pull_request_ref(&ctx_b, &pr_ref).await?;
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
    let task_a = create_task(&service, &ctx, "Tenant isolation test").await?;
    let ctx_b = ctx_other_tenant(&ctx);
    let task_b = create_task(&service, &ctx_b, "Same issue different tenant").await?;

    // Each tenant's index must resolve to its own task.
    let issue_ref = IssueRef::from_parts(PROVIDER, REPO, ISSUE_NO)?;

    let found_a = service
        .find_by_issue_ref(&ctx, &issue_ref)
        .await?
        .expect("tenant A must still find its task");
    assert_eq!(found_a.id(), task_a.id());

    let found_b = service
        .find_by_issue_ref(&ctx_b, &issue_ref)
        .await?
        .expect("tenant B must find its task");
    assert_eq!(found_b.id(), task_b.id());

    assert_ne!(
        task_a.id(),
        task_b.id(),
        "tasks must be distinct across tenants"
    );
    Ok(())
}
