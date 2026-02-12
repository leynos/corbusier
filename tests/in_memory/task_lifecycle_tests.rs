//! In-memory integration tests for task lifecycle operations.

use std::sync::Arc;

use corbusier::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::{BranchRef, IssueRef, PullRequestRef, TaskState},
    ports::TaskRepositoryError,
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TaskLifecycleService,
    },
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

type TestService = TaskLifecycleService<InMemoryTaskRepository, DefaultClock>;

#[fixture]
fn service() -> TestService {
    TaskLifecycleService::new(
        Arc::new(InMemoryTaskRepository::new()),
        Arc::new(DefaultClock),
    )
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn create_and_lookup_by_external_issue_reference(service: TestService) {
    let request = CreateTaskFromIssueRequest::new(
        "github",
        "corbusier/core",
        211,
        "Implement issue to task mapping",
    )
    .with_labels(vec!["feature".to_owned(), "task-lifecycle".to_owned()]);

    let created = service
        .create_from_issue(request)
        .await
        .expect("task creation should succeed");
    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 211).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&issue_ref)
        .await
        .expect("lookup should succeed");

    assert_eq!(found, Some(created));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_issue_reference_is_rejected(service: TestService) {
    service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "gitlab",
            "corbusier/core",
            99,
            "First task",
        ))
        .await
        .expect("first task creation should succeed");

    let duplicate_result = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "gitlab",
            "corbusier/core",
            99,
            "Duplicate task",
        ))
        .await;

    assert!(matches!(
        duplicate_result,
        Err(corbusier::task::services::TaskLifecycleError::Repository(
            TaskRepositoryError::DuplicateIssueOrigin(_)
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn lookup_returns_none_for_missing_reference(service: TestService) {
    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 404).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&issue_ref)
        .await
        .expect("lookup should succeed");
    assert!(found.is_none());
}

// ── Branch and PR association integration tests ─────────────────────

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_branch_and_retrieve_by_ref(service: TestService) {
    let task = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            300,
            "Branch association test",
        ))
        .await
        .expect("task creation should succeed");
    let updated = service
        .associate_branch(AssociateBranchRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            "feature/branch-integ",
        ))
        .await
        .expect("branch association should succeed");
    assert!(updated.branch_ref().is_some());

    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "feature/branch-integ")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&branch_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 1);
    assert_eq!(found.first().expect("at least one task").id(), task.id());
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_pr_and_verify_state_transition(service: TestService) {
    let task = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            301,
            "PR association test",
        ))
        .await
        .expect("task creation should succeed");
    let updated = service
        .associate_pull_request(AssociatePullRequestRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            55,
        ))
        .await
        .expect("PR association should succeed");
    assert!(updated.pull_request_ref().is_some());
    assert_eq!(updated.state(), TaskState::InReview);

    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 55).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&pr_ref)
        .await
        .expect("lookup should succeed");
    assert_eq!(found.len(), 1);
    assert_eq!(found.first().expect("at least one task").id(), task.id());
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn multiple_tasks_sharing_branch_all_returned(service: TestService) {
    let task1 = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            302,
            "Task 1",
        ))
        .await
        .expect("first task creation should succeed");
    let task2 = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            303,
            "Task 2",
        ))
        .await
        .expect("second task creation should succeed");

    service
        .associate_branch(AssociateBranchRequest::new(
            task1.id(),
            "github",
            "corbusier/core",
            "shared/integration-branch",
        ))
        .await
        .expect("first task branch association should succeed");
    service
        .associate_branch(AssociateBranchRequest::new(
            task2.id(),
            "github",
            "corbusier/core",
            "shared/integration-branch",
        ))
        .await
        .expect("second task branch association should succeed");

    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "shared/integration-branch")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&branch_ref)
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
