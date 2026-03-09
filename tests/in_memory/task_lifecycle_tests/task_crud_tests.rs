//! Tests for task creation, lookup, and duplicate detection.

use super::{TestService, service};
use crate::in_memory::helpers::ctx;
use corbusier::context::RequestContext;
use corbusier::task::{
    domain::IssueRef, ports::TaskRepositoryError, services::CreateTaskFromIssueRequest,
};
use rstest::rstest;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn create_and_lookup_by_external_issue_reference(service: TestService, ctx: RequestContext) {
    let request = CreateTaskFromIssueRequest::new(
        "github",
        "corbusier/core",
        211,
        "Implement issue to task mapping",
    )
    .with_labels(vec!["feature".to_owned(), "task-lifecycle".to_owned()]);

    let created = service
        .create_from_issue(&ctx, request)
        .await
        .expect("task creation should succeed");
    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 211).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&ctx, &issue_ref)
        .await
        .expect("lookup should succeed");

    assert_eq!(found, Some(created));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_issue_reference_is_rejected(service: TestService, ctx: RequestContext) {
    service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("gitlab", "corbusier/core", 99, "First task"),
        )
        .await
        .expect("first task creation should succeed");

    let duplicate_result = service
        .create_from_issue(
            &ctx,
            CreateTaskFromIssueRequest::new("gitlab", "corbusier/core", 99, "Duplicate task"),
        )
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
async fn lookup_returns_none_for_missing_reference(service: TestService, ctx: RequestContext) {
    let issue_ref =
        IssueRef::from_parts("github", "corbusier/core", 404).expect("valid issue reference");
    let found = service
        .find_by_issue_ref(&ctx, &issue_ref)
        .await
        .expect("lookup should succeed");
    assert!(found.is_none());
}
