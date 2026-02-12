//! Service orchestration tests for task lifecycle operations.

use std::sync::Arc;

use crate::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::{BranchRef, IssueRef, PullRequestRef, Task, TaskDomainError, TaskId, TaskState},
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
    assert_eq!(found.len(), 1);
    match found.first() {
        Some(task) => assert_eq!(task.id(), expected_id),
        None => panic!("expected at least one task"),
    }
}

async fn assert_create_from_issue_fails_with_domain_error(
    service: &TestService,
    request: CreateTaskFromIssueRequest,
    error_matcher: impl FnOnce(&TaskDomainError) -> bool,
    error_description: &str,
) {
    let result = service.create_from_issue(request).await;

    match result {
        Err(TaskLifecycleError::Domain(ref error)) if error_matcher(error) => {}
        other => panic!("expected {error_description}, got {other:?}"),
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

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn create_from_issue_persists_and_is_retrievable(service: TestService) {
    let request = CreateTaskFromIssueRequest::new(
        "github",
        "owner/repo",
        123,
        "Implement lifecycle persistence",
    )
    .with_description("Map issue metadata to task origin")
    .with_labels(vec!["feature".to_owned(), "roadmap-1.2.1".to_owned()])
    .with_assignees(vec!["alice".to_owned()])
    .with_milestone("Phase 1");

    let created = service
        .create_from_issue(request)
        .await
        .expect("task creation should succeed");
    let issue_ref =
        IssueRef::from_parts("github", "owner/repo", 123).expect("valid issue reference");
    let fetched = service
        .find_by_issue_ref(&issue_ref)
        .await
        .expect("lookup should succeed");

    assert_eq!(fetched, Some(created));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn create_from_issue_rejects_duplicate_issue_reference(service: TestService) {
    let first = CreateTaskFromIssueRequest::new("gitlab", "team/repo", 77, "Initial task");
    service
        .create_from_issue(first)
        .await
        .expect("first task creation should succeed");

    let duplicate = CreateTaskFromIssueRequest::new("gitlab", "team/repo", 77, "Duplicate");
    let result = service.create_from_issue(duplicate).await;

    let Err(TaskLifecycleError::Repository(TaskRepositoryError::DuplicateIssueOrigin(issue_ref))) =
        result
    else {
        panic!("expected duplicate issue origin error");
    };

    assert_eq!(issue_ref.provider().as_str(), "gitlab");
    assert_eq!(issue_ref.repository().as_str(), "team/repo");
    assert_eq!(issue_ref.issue_number().value(), 77);
}

#[rstest]
#[case(
    CreateTaskFromIssueRequest::new("unknown-provider", "owner/repo", 10, "Invalid provider"),
    TaskDomainError::InvalidIssueProvider("unknown-provider".to_owned()),
    "InvalidIssueProvider domain error"
)]
#[case(
    CreateTaskFromIssueRequest::new("github", "owner-only", 10, "Invalid repository"),
    TaskDomainError::InvalidRepository("owner-only".to_owned()),
    "InvalidRepository domain error"
)]
#[tokio::test(flavor = "multi_thread")]
async fn create_from_issue_rejects_invalid_issue_metadata(
    service: TestService,
    #[case] request: CreateTaskFromIssueRequest,
    #[case] expected_error: TaskDomainError,
    #[case] error_description: &str,
) {
    assert_create_from_issue_fails_with_domain_error(
        &service,
        request,
        |error| error == &expected_error,
        error_description,
    )
    .await;
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn create_from_issue_rejects_empty_title(service: TestService) {
    let request = CreateTaskFromIssueRequest::new("github", "owner/repo", 10, "   ");

    assert_create_from_issue_fails_with_domain_error(
        &service,
        request,
        |error| matches!(error, TaskDomainError::EmptyIssueTitle),
        "EmptyIssueTitle domain error",
    )
    .await;
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn find_by_issue_ref_returns_none_when_missing(service: TestService) {
    let issue_ref =
        IssueRef::from_parts("github", "missing/repo", 808).expect("valid issue reference");
    let fetched = service
        .find_by_issue_ref(&issue_ref)
        .await
        .expect("lookup should succeed");
    assert!(fetched.is_none());
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
