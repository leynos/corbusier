//! Tests for branch and pull request association.

use super::{TestService, assert_single_task_found, service};
use crate::in_memory::helpers::ctx;
use corbusier::context::RequestContext;
use corbusier::task::{
    domain::{BranchRef, PullRequestRef, TaskState},
    services::{AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest},
};
use rstest::rstest;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_branch_and_retrieve_by_ref(
    service: TestService,
    ctx: RequestContext,
) -> Result<(), eyre::Report> {
    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new(
                "github",
                "corbusier/core",
                300,
                "Branch association test",
            ),
        )
        .await
        .expect("task creation should succeed");
    let updated = service
        .associate_branch(
            &ctx,
            AssociateBranchRequest::new(
                task.id(),
                "github",
                "corbusier/core",
                "feature/branch-integ",
            ),
        )
        .await
        .expect("branch association should succeed");
    assert!(updated.branch_ref().is_some());

    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "feature/branch-integ")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&ctx, &branch_ref)
        .await
        .expect("lookup should succeed");
    assert_single_task_found(&found, task.id())?;
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_pr_and_verify_state_transition(
    service: TestService,
    ctx: RequestContext,
) -> Result<(), eyre::Report> {
    let task = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("github", "corbusier/core", 301, "PR association test"),
        )
        .await
        .expect("task creation should succeed");
    let updated = service
        .associate_pull_request(
            &ctx,
            AssociatePullRequestRequest::new(task.id(), "github", "corbusier/core", 55),
        )
        .await
        .expect("PR association should succeed");
    assert!(updated.pull_request_ref().is_some());
    assert_eq!(updated.state(), TaskState::InReview);

    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 55).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&ctx, &pr_ref)
        .await
        .expect("lookup should succeed");
    assert_single_task_found(&found, task.id())?;
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn multiple_tasks_sharing_branch_all_returned(service: TestService, ctx: RequestContext) {
    let task1 = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("github", "corbusier/core", 302, "Task 1"),
        )
        .await
        .expect("first task creation should succeed");
    let task2 = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("github", "corbusier/core", 303, "Task 2"),
        )
        .await
        .expect("second task creation should succeed");

    service
        .associate_branch(
            &ctx,
            AssociateBranchRequest::new(
                task1.id(),
                "github",
                "corbusier/core",
                "shared/integration-branch",
            ),
        )
        .await
        .expect("first task branch association should succeed");
    service
        .associate_branch(
            &ctx,
            AssociateBranchRequest::new(
                task2.id(),
                "github",
                "corbusier/core",
                "shared/integration-branch",
            ),
        )
        .await
        .expect("second task branch association should succeed");

    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "shared/integration-branch")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&ctx, &branch_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 2);
    let ids: Vec<_> = found
        .iter()
        .map(corbusier::task::domain::Task::id)
        .collect();
    assert!(ids.contains(&task1.id()));
    assert!(ids.contains(&task2.id()));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn multiple_tasks_sharing_pull_request_all_returned(
    service: TestService,
    ctx: RequestContext,
) {
    let task1 = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("github", "corbusier/core", 304, "PR share 1"),
        )
        .await
        .expect("first task creation should succeed");
    let task2 = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("github", "corbusier/core", 305, "PR share 2"),
        )
        .await
        .expect("second task creation should succeed");

    service
        .associate_pull_request(
            &ctx,
            AssociatePullRequestRequest::new(task1.id(), "github", "corbusier/core", 77),
        )
        .await
        .expect("first task PR association should succeed");
    service
        .associate_pull_request(
            &ctx,
            AssociatePullRequestRequest::new(task2.id(), "github", "corbusier/core", 77),
        )
        .await
        .expect("second task PR association should succeed");

    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 77).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&ctx, &pr_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 2);
    let ids: Vec<_> = found
        .iter()
        .map(corbusier::task::domain::Task::id)
        .collect();
    assert!(ids.contains(&task1.id()));
    assert!(ids.contains(&task2.id()));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_by_pull_request_ref_returns_empty_when_none_match(
    service: TestService,
    ctx: RequestContext,
) {
    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 999).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&ctx, &pr_ref)
        .await
        .expect("lookup should succeed");
    assert!(
        found.is_empty(),
        "expected empty result for unmatched PR ref"
    );
}
