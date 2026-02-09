//! Domain-focused tests for issue-to-task mapping behaviour.

use crate::task::domain::{
    ExternalIssue, ExternalIssueMetadata, IssueRef, Task, TaskDomainError, TaskState,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

#[rstest]
fn issue_ref_from_parts_accepts_valid_values() {
    let issue_ref = IssueRef::from_parts("github", "owner/repo", 42).expect("valid issue ref");

    assert_eq!(issue_ref.provider().as_str(), "github");
    assert_eq!(issue_ref.repository().as_str(), "owner/repo");
    assert_eq!(issue_ref.issue_number().value(), 42);
}

#[rstest]
fn issue_ref_from_parts_rejects_invalid_repository() {
    let result = IssueRef::from_parts("github", "owner-only", 42);
    assert_eq!(
        result,
        Err(TaskDomainError::InvalidRepository("owner-only".to_owned()))
    );
}

#[rstest]
fn issue_ref_from_parts_rejects_zero_issue_number() {
    let result = IssueRef::from_parts("github", "owner/repo", 0);
    assert_eq!(result, Err(TaskDomainError::InvalidIssueNumber(0)));
}

#[rstest]
fn issue_metadata_rejects_empty_title() {
    let result = ExternalIssueMetadata::new("    ");
    assert_eq!(result, Err(TaskDomainError::EmptyIssueTitle));
}

#[rstest]
fn task_new_from_issue_sets_draft_state_and_timestamps(clock: DefaultClock) {
    let issue_ref = IssueRef::from_parts("github", "owner/repo", 7).expect("valid issue ref");
    let external_metadata = ExternalIssueMetadata::new("Fix parser edge case")
        .expect("valid issue metadata")
        .with_description("Ensure parser handles escaped delimiters")
        .with_labels(vec!["bug".to_owned(), "priority-high".to_owned()])
        .with_assignees(vec!["alice".to_owned()])
        .with_milestone("M1");
    let task = Task::new_from_issue(
        &ExternalIssue::new(issue_ref.clone(), external_metadata),
        &clock,
    );

    assert_eq!(task.state(), TaskState::Draft);
    assert_eq!(task.origin().issue_ref(), &issue_ref);
    assert_eq!(task.created_at(), task.updated_at());

    let crate::task::domain::TaskOrigin::Issue {
        metadata: issue_metadata,
        ..
    } = task.origin();
    assert_eq!(issue_metadata.title, "Fix parser edge case");
    assert_eq!(
        issue_metadata.description.as_deref(),
        Some("Ensure parser handles escaped delimiters")
    );
    assert_eq!(
        issue_metadata.labels,
        vec!["bug".to_owned(), "priority-high".to_owned()]
    );
    assert_eq!(issue_metadata.assignees, vec!["alice".to_owned()]);
    assert_eq!(issue_metadata.milestone.as_deref(), Some("M1"));
}
