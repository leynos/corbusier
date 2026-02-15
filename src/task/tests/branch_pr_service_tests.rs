//! Service orchestration tests for branch and pull request association.

use std::sync::Arc;

use crate::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::{BranchRef, PullRequestRef, Task, TaskDomainError, TaskId, TaskState},
    ports::TaskRepositoryError,
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TaskLifecycleError, TaskLifecycleService,
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

async fn create_test_task(
    service: &TestService,
    issue_number: u32,
    title: &str,
) -> Result<Task, TaskLifecycleError> {
    let request =
        CreateTaskFromIssueRequest::new("github", "owner/repo", u64::from(issue_number), title);
    service.create_from_issue(request).await
}

fn assert_not_found_error<T: std::fmt::Debug>(result: Result<T, TaskLifecycleError>) {
    match result {
        Err(TaskLifecycleError::Repository(TaskRepositoryError::NotFound(_))) => {}
        other => panic!("expected NotFound repository error, got {other:?}"),
    }
}

fn assert_branch_already_associated_error<T: std::fmt::Debug>(
    result: Result<T, TaskLifecycleError>,
) {
    match result {
        Err(TaskLifecycleError::Domain(TaskDomainError::BranchAlreadyAssociated(_))) => {}
        other => panic!("expected BranchAlreadyAssociated domain error, got {other:?}"),
    }
}

fn assert_pr_already_associated_error<T: std::fmt::Debug>(result: Result<T, TaskLifecycleError>) {
    match result {
        Err(TaskLifecycleError::Domain(TaskDomainError::PullRequestAlreadyAssociated(_))) => {}
        other => panic!("expected PullRequestAlreadyAssociated domain error, got {other:?}"),
    }
}

fn assert_single_task_found(found: &[Task], expected_id: TaskId) {
    assert_eq!(found.len(), 1, "expected exactly one task");
    if let Some(task) = found.first() {
        assert_eq!(task.id(), expected_id, "task ID should match");
    }
}

/// Helper to test that duplicate associations are rejected.
async fn assert_duplicate_association_rejected<F1, F2, Fut1, Fut2, E>(
    task_id: TaskId,
    first_association: F1,
    duplicate_association: F2,
    assert_error: E,
) where
    F1: FnOnce(TaskId) -> Fut1,
    F2: FnOnce(TaskId) -> Fut2,
    Fut1: std::future::Future<Output = Result<Task, TaskLifecycleError>>,
    Fut2: std::future::Future<Output = Result<Task, TaskLifecycleError>>,
    E: FnOnce(Result<Task, TaskLifecycleError>),
{
    first_association(task_id)
        .await
        .expect("first association should succeed");

    let result = duplicate_association(task_id).await;

    assert_error(result);
}

// ── Branch association tests ────────────────────────────────────────

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_branch_persists_and_is_retrievable(service: TestService) {
    let task = create_test_task(&service, 500, "Task for branch test")
        .await
        .expect("task creation should succeed");
    let request =
        AssociateBranchRequest::new(task.id(), "github", "owner/repo", "feature/branch-test");

    let updated = service
        .associate_branch(request)
        .await
        .expect("branch association should succeed");

    assert!(updated.branch_ref().is_some());

    let branch_ref = BranchRef::from_parts("github", "owner/repo", "feature/branch-test")
        .expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&branch_ref)
        .await
        .expect("lookup should succeed");
    assert_single_task_found(&found, task.id());
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_branch_rejects_duplicate_on_same_task(service: TestService) {
    let task = create_test_task(&service, 501, "Task")
        .await
        .expect("task creation should succeed");

    assert_duplicate_association_rejected(
        task.id(),
        |task_id| {
            service.associate_branch(AssociateBranchRequest::new(
                task_id,
                "github",
                "owner/repo",
                "branch-1",
            ))
        },
        |task_id| {
            service.associate_branch(AssociateBranchRequest::new(
                task_id,
                "github",
                "owner/repo",
                "branch-2",
            ))
        },
        assert_branch_already_associated_error,
    )
    .await;
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_branch_returns_not_found_for_unknown_task(service: TestService) {
    let unknown_id = TaskId::new();
    let request = AssociateBranchRequest::new(unknown_id, "github", "owner/repo", "main");

    let result = service.associate_branch(request).await;

    assert_not_found_error(result);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_branch_rejects_invalid_branch_name(service: TestService) {
    let task = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "owner/repo",
            502,
            "Task",
        ))
        .await
        .expect("task creation should succeed");
    let request = AssociateBranchRequest::new(task.id(), "github", "owner/repo", "invalid:branch");

    let result = service.associate_branch(request).await;

    assert!(matches!(
        result,
        Err(TaskLifecycleError::Domain(
            TaskDomainError::InvalidBranchName(_)
        ))
    ));
}

// ── Pull request association tests ──────────────────────────────────

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_pull_request_persists_and_transitions_to_in_review(service: TestService) {
    let task = create_test_task(&service, 600, "Task")
        .await
        .expect("task creation should succeed");
    let request = AssociatePullRequestRequest::new(task.id(), "github", "owner/repo", 42);

    let updated = service
        .associate_pull_request(request)
        .await
        .expect("PR association should succeed");

    assert!(updated.pull_request_ref().is_some());
    assert_eq!(updated.state(), TaskState::InReview);

    let pr_ref = PullRequestRef::from_parts("github", "owner/repo", 42).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&pr_ref)
        .await
        .expect("lookup should succeed");
    assert_single_task_found(&found, task.id());
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_pull_request_rejects_duplicate_on_same_task(service: TestService) {
    let task = create_test_task(&service, 601, "Task")
        .await
        .expect("task creation should succeed");

    assert_duplicate_association_rejected(
        task.id(),
        |task_id| {
            service.associate_pull_request(AssociatePullRequestRequest::new(
                task_id,
                "github",
                "owner/repo",
                10,
            ))
        },
        |task_id| {
            service.associate_pull_request(AssociatePullRequestRequest::new(
                task_id,
                "github",
                "owner/repo",
                20,
            ))
        },
        assert_pr_already_associated_error,
    )
    .await;
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn associate_pull_request_returns_not_found_for_unknown_task(service: TestService) {
    let unknown_id = TaskId::new();
    let request = AssociatePullRequestRequest::new(unknown_id, "github", "owner/repo", 1);

    let result = service.associate_pull_request(request).await;

    assert_not_found_error(result);
}

// ── Many-to-many branch sharing ─────────────────────────────────────

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn multiple_tasks_sharing_branch_all_returned(service: TestService) {
    let task1 = create_test_task(&service, 700, "Task 1")
        .await
        .expect("first task creation should succeed");
    let task2 = create_test_task(&service, 701, "Task 2")
        .await
        .expect("second task creation should succeed");

    service
        .associate_branch(AssociateBranchRequest::new(
            task1.id(),
            "github",
            "owner/repo",
            "shared/branch",
        ))
        .await
        .expect("first task branch association should succeed");

    service
        .associate_branch(AssociateBranchRequest::new(
            task2.id(),
            "github",
            "owner/repo",
            "shared/branch",
        ))
        .await
        .expect("second task branch association should succeed");

    let branch_ref =
        BranchRef::from_parts("github", "owner/repo", "shared/branch").expect("valid branch ref");
    let found = service
        .find_by_branch_ref(&branch_ref)
        .await
        .expect("lookup should succeed");

    assert_eq!(found.len(), 2);
    let ids: Vec<_> = found.iter().map(Task::id).collect();
    assert!(ids.contains(&task1.id()));
    assert!(ids.contains(&task2.id()));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn multiple_tasks_sharing_pull_request_all_returned(service: TestService) {
    let task1 = create_test_task(&service, 800, "PR share 1")
        .await
        .expect("first task creation should succeed");
    let task2 = create_test_task(&service, 801, "PR share 2")
        .await
        .expect("second task creation should succeed");

    service
        .associate_pull_request(AssociatePullRequestRequest::new(
            task1.id(),
            "github",
            "owner/repo",
            99,
        ))
        .await
        .expect("first task PR association should succeed");

    service
        .associate_pull_request(AssociatePullRequestRequest::new(
            task2.id(),
            "github",
            "owner/repo",
            99,
        ))
        .await
        .expect("second task PR association should succeed");

    let pr_ref = PullRequestRef::from_parts("github", "owner/repo", 99).expect("valid PR ref");
    let found = service
        .find_by_pull_request_ref(&pr_ref)
        .await
        .expect("lookup should succeed");

    assert_eq!(found.len(), 2);
    let ids: Vec<_> = found.iter().map(Task::id).collect();
    assert!(ids.contains(&task1.id()));
    assert!(ids.contains(&task2.id()));
}
