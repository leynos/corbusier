//! In-memory integration tests for issue-to-task creation and lookup.

use std::sync::Arc;

use corbusier::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::IssueRef,
    ports::TaskRepositoryError,
    services::{CreateTaskFromIssueRequest, TaskLifecycleService},
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
