//! Tests for cross-tenant task isolation.

use super::{TestService, service};
use crate::in_memory::helpers::{ctx, other_ctx};
use corbusier::context::RequestContext;
use corbusier::task::{
    domain::IssueRef,
    ports::TaskRepositoryError,
    services::{
        AssociateBranchRequest, CreateTaskFromIssueRequest, TaskLifecycleError,
        TransitionTaskRequest,
    },
};
use rstest::rstest;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn lookup_is_scoped_to_tenant(service: TestService, ctx: RequestContext) {
    let ctx_b = other_ctx();

    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                900,
                "Tenant isolation test",
            ),
        )
        .await
        .expect("task creation under tenant A should succeed");

    // find_by_issue_ref with tenant B context returns None.
    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 900).expect("valid issue reference");
    let found_b = service
        .find_by_issue_ref(&ctx_b, &issue_ref)
        .await
        .expect("lookup under tenant B should succeed");
    assert!(
        found_b.is_none(),
        "tenant B must not see tenant A tasks via issue ref"
    );

    // Transition with tenant B context returns NotFound.
    let transition_result = service
        .transition_task(&ctx_b, TransitionTaskRequest::new(task.id(), "in_progress"))
        .await;
    assert!(
        matches!(
            transition_result,
            Err(TaskLifecycleError::Repository(
                TaskRepositoryError::NotFound(_)
            ))
        ),
        "transition under tenant B should fail with NotFound"
    );

    // associate_branch with tenant B context returns NotFound.
    let branch_result = service
        .associate_branch(
            &ctx_b,
            AssociateBranchRequest::new(
                task.id(),
                "github",
                "corbusier/core",
                "feature/cross-tenant",
            ),
        )
        .await;
    assert!(
        matches!(
            branch_result,
            Err(TaskLifecycleError::Repository(
                TaskRepositoryError::NotFound(_)
            ))
        ),
        "branch association under tenant B should fail with NotFound"
    );

    // Same issue ref under tenant B is allowed (no cross-tenant duplicate).
    service
        .create_from_issue(
            &ctx_b,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                900,
                "Same issue different tenant",
            ),
        )
        .await
        .expect("same issue ref under tenant B should succeed");
}
