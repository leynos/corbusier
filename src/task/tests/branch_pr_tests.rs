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

fn create_test_task(clock: &DefaultClock) -> Task {
    let issue_ref = IssueRef::from_parts("github", "owner/repo", 1).expect("valid issue ref");
    let metadata = ExternalIssueMetadata::new("Test task").expect("valid metadata");
    Task::new_from_issue(&ExternalIssue::new(issue_ref, metadata), clock)
}

#[rstest]
fn associate_branch_sets_ref_and_updates_timestamp(clock: DefaultClock) {
    let mut task = create_test_task(&clock);
    let original_updated_at = task.updated_at();

    let branch =
        BranchRef::from_parts("github", "owner/repo", "feature/x").expect("valid branch ref");
    task.associate_branch(branch.clone(), &clock)
        .expect("association should succeed");

    assert_eq!(task.branch_ref(), Some(&branch));
    assert!(task.updated_at() >= original_updated_at);
    assert_eq!(task.state(), TaskState::Draft);
}

#[rstest]
fn associate_branch_rejects_when_already_set(clock: DefaultClock) {
    let mut task = create_test_task(&clock);
    let branch1 =
        BranchRef::from_parts("github", "owner/repo", "branch-1").expect("valid branch ref");
    task.associate_branch(branch1, &clock)
        .expect("first association should succeed");

    let branch2 =
        BranchRef::from_parts("github", "owner/repo", "branch-2").expect("valid branch ref");
    let result = task.associate_branch(branch2, &clock);

    assert!(matches!(
        result,
        Err(TaskDomainError::BranchAlreadyAssociated(_))
    ));
}

// ── Task::associate_pull_request ────────────────────────────────────

#[rstest]
fn associate_pull_request_sets_ref_transitions_to_in_review(clock: DefaultClock) {
    let mut task = create_test_task(&clock);
    let original_updated_at = task.updated_at();

    let pr = PullRequestRef::from_parts("github", "owner/repo", 42).expect("valid PR ref");
    task.associate_pull_request(pr.clone(), &clock)
        .expect("association should succeed");

    assert_eq!(task.pull_request_ref(), Some(&pr));
    assert_eq!(task.state(), TaskState::InReview);
    assert!(task.updated_at() >= original_updated_at);
}

#[rstest]
fn associate_pull_request_rejects_when_already_set(clock: DefaultClock) {
    let mut task = create_test_task(&clock);
    let pr1 = PullRequestRef::from_parts("github", "owner/repo", 1).expect("valid PR ref");
    task.associate_pull_request(pr1, &clock)
        .expect("first association should succeed");

    let pr2 = PullRequestRef::from_parts("github", "owner/repo", 2).expect("valid PR ref");
    let result = task.associate_pull_request(pr2, &clock);

    assert!(matches!(
        result,
        Err(TaskDomainError::PullRequestAlreadyAssociated(_))
    ));
}
