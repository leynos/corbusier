//! Service orchestration tests for task lifecycle operations.

use std::sync::Arc;

use crate::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::{IssueRef, TaskDomainError},
    ports::TaskRepositoryError,
    services::{CreateTaskFromIssueRequest, TaskLifecycleError, TaskLifecycleService},
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
