//! Domain tests for branch and pull request value objects and task association.

use crate::task::domain::{
    BranchName, BranchRef, ExternalIssue, ExternalIssueMetadata, IssueRef, PullRequestNumber,
    PullRequestRef, Task, TaskDomainError, TaskState,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

// ── BranchName ──────────────────────────────────────────────────────

#[rstest]
#[case("feature/my-branch")]
#[case("main")]
#[case("fix/issue-42")]
fn branch_name_accepts_valid_names(#[case] name: &str) {
    let branch = BranchName::new(name).expect("valid branch name");
    assert_eq!(branch.as_str(), name);
}

#[rstest]
fn branch_name_trims_whitespace() {
    let branch = BranchName::new("  feature/x  ").expect("valid branch name");
    assert_eq!(branch.as_str(), "feature/x");
}

#[rstest]
#[case("")]
#[case("   ")]
fn branch_name_rejects_empty(#[case] name: &str) {
    assert!(matches!(
        BranchName::new(name),
        Err(TaskDomainError::InvalidBranchName(_))
    ));
}

#[rstest]
fn branch_name_rejects_colon() {
    assert!(matches!(
        BranchName::new("feat:ure"),
        Err(TaskDomainError::InvalidBranchName(_))
    ));
}

#[rstest]
fn branch_name_rejects_overlength() {
    let long_name = "a".repeat(201);
    assert!(matches!(
        BranchName::new(long_name),
        Err(TaskDomainError::InvalidBranchName(_))
    ));
}

#[rstest]
fn branch_name_accepts_max_length() {
    let max_name = "a".repeat(200);
    let branch = BranchName::new(max_name.clone()).expect("max length branch name");
    assert_eq!(branch.as_str(), max_name);
}

// ── BranchRef ───────────────────────────────────────────────────────

#[rstest]
fn branch_ref_from_parts_accepts_valid() {
    let branch =
        BranchRef::from_parts("github", "owner/repo", "feature/x").expect("valid branch ref");
    assert_eq!(branch.provider().as_str(), "github");
    assert_eq!(branch.repository().as_str(), "owner/repo");
    assert_eq!(branch.branch_name().as_str(), "feature/x");
}

#[rstest]
fn branch_ref_from_parts_rejects_invalid_provider() {
    let result = BranchRef::from_parts("unknown", "owner/repo", "main");
    assert!(result.is_err());
}

#[rstest]
fn branch_ref_from_parts_rejects_invalid_repository() {
    let result = BranchRef::from_parts("github", "no-slash", "main");
    assert!(result.is_err());
}

#[rstest]
fn branch_ref_from_parts_rejects_invalid_branch_name() {
    let result = BranchRef::from_parts("github", "owner/repo", "bad:name");
    assert!(result.is_err());
}

#[rstest]
fn branch_ref_round_trips_through_display_and_try_from() {
    let original =
        BranchRef::from_parts("github", "owner/repo", "feature/x").expect("valid branch ref");
    let canonical = original.to_string();
    assert_eq!(canonical, "github:owner/repo:feature/x");

    let parsed = BranchRef::try_from(canonical.as_str()).expect("canonical should parse");
    assert_eq!(parsed, original);
}

#[rstest]
fn branch_ref_parse_canonical_rejects_malformed() {
    assert!(BranchRef::parse_canonical("no-colons-here").is_err());
    assert!(BranchRef::parse_canonical("only:one").is_err());
}

// ── PullRequestNumber ───────────────────────────────────────────────

#[rstest]
#[case(1)]
#[case(42)]
#[case(i64::MAX as u64)]
fn pull_request_number_accepts_valid(#[case] n: u64) {
    let prn = PullRequestNumber::new(n).expect("valid PR number");
    assert_eq!(prn.value(), n);
}

#[rstest]
fn pull_request_number_rejects_zero() {
    assert!(matches!(
        PullRequestNumber::new(0),
        Err(TaskDomainError::InvalidPullRequestNumber(0))
    ));
}

#[rstest]
fn pull_request_number_rejects_beyond_i64_max() {
    let too_large = (i64::MAX as u64) + 1;
    assert!(matches!(
        PullRequestNumber::new(too_large),
        Err(TaskDomainError::InvalidPullRequestNumber(n)) if n == too_large
    ));
}

// ── PullRequestRef ──────────────────────────────────────────────────

#[rstest]
fn pull_request_ref_from_parts_accepts_valid() {
    let pr = PullRequestRef::from_parts("github", "owner/repo", 42).expect("valid PR ref");
    assert_eq!(pr.provider().as_str(), "github");
    assert_eq!(pr.repository().as_str(), "owner/repo");
    assert_eq!(pr.pull_request_number().value(), 42);
}

#[rstest]
fn pull_request_ref_round_trips_through_display_and_try_from() {
    let original = PullRequestRef::from_parts("gitlab", "team/project", 99).expect("valid PR ref");
    let canonical = original.to_string();
    assert_eq!(canonical, "gitlab:team/project:99");

    let parsed = PullRequestRef::try_from(canonical.as_str()).expect("canonical should parse");
    assert_eq!(parsed, original);
}

#[rstest]
fn pull_request_ref_parse_canonical_rejects_malformed() {
    assert!(PullRequestRef::parse_canonical("no-colons").is_err());
    assert!(PullRequestRef::parse_canonical("github:owner/repo:notanumber").is_err());
}

// ── Task::associate_branch ──────────────────────────────────────────

#[fixture]
fn test_task(clock: DefaultClock) -> Result<Task, TaskDomainError> {
    let issue_ref = IssueRef::from_parts("github", "owner/repo", 1)?;
    let metadata = ExternalIssueMetadata::new("Test task")?;
    Ok(Task::new_from_issue(
        &ExternalIssue::new(issue_ref, metadata),
        &clock,
    ))
}

#[rstest]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test uses assertions for verification while returning Result for error propagation"
)]
fn associate_branch_sets_ref_and_updates_timestamp(
    #[from(test_task)] task_result: Result<Task, TaskDomainError>,
    clock: DefaultClock,
) -> Result<(), TaskDomainError> {
    let mut test_task = task_result?;
    let original_updated_at = test_task.updated_at();

    let branch = BranchRef::from_parts("github", "owner/repo", "feature/x")?;
    test_task.associate_branch(branch.clone(), &clock)?;

    assert_eq!(test_task.branch_ref(), Some(&branch));
    assert!(test_task.updated_at() >= original_updated_at);
    assert_eq!(test_task.state(), TaskState::Draft);
    Ok(())
}

#[rstest]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test uses assertions for verification while returning Result for error propagation"
)]
fn associate_branch_rejects_when_already_set(
    #[from(test_task)] task_result: Result<Task, TaskDomainError>,
    clock: DefaultClock,
) -> Result<(), TaskDomainError> {
    let mut test_task = task_result?;
    let branch1 = BranchRef::from_parts("github", "owner/repo", "branch-1")?;
    test_task.associate_branch(branch1, &clock)?;

    let branch2 = BranchRef::from_parts("github", "owner/repo", "branch-2")?;
    let result = test_task.associate_branch(branch2, &clock);

    assert!(matches!(
        result,
        Err(TaskDomainError::BranchAlreadyAssociated(_))
    ));
    Ok(())
}

// ── Task::associate_pull_request ────────────────────────────────────

#[rstest]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test uses assertions for verification while returning Result for error propagation"
)]
fn associate_pull_request_sets_ref_transitions_to_in_review(
    #[from(test_task)] task_result: Result<Task, TaskDomainError>,
    clock: DefaultClock,
) -> Result<(), TaskDomainError> {
    let mut test_task = task_result?;
    let original_updated_at = test_task.updated_at();

    let pr = PullRequestRef::from_parts("github", "owner/repo", 42)?;
    test_task.associate_pull_request(pr.clone(), &clock)?;

    assert_eq!(test_task.pull_request_ref(), Some(&pr));
    assert_eq!(test_task.state(), TaskState::InReview);
    assert!(test_task.updated_at() >= original_updated_at);
    Ok(())
}

#[rstest]
#[expect(
    clippy::panic_in_result_fn,
    reason = "Test uses assertions for verification while returning Result for error propagation"
)]
fn associate_pull_request_rejects_when_already_set(
    #[from(test_task)] task_result: Result<Task, TaskDomainError>,
    clock: DefaultClock,
) -> Result<(), TaskDomainError> {
    let mut test_task = task_result?;
    let pr1 = PullRequestRef::from_parts("github", "owner/repo", 1)?;
    test_task.associate_pull_request(pr1, &clock)?;

    let pr2 = PullRequestRef::from_parts("github", "owner/repo", 2)?;
    let result = test_task.associate_pull_request(pr2, &clock);

    assert!(matches!(
        result,
        Err(TaskDomainError::PullRequestAlreadyAssociated(_))
    ));
    Ok(())
}
