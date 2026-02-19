//! Unit tests for task state transition validation.

use crate::task::domain::{
    ExternalIssue, ExternalIssueMetadata, IssueRef, PullRequestRef, Task, TaskDomainError,
    TaskState,
};
use eyre::{bail, ensure};
use mockable::DefaultClock;
use rstest::{fixture, rstest};

const ALL_STATES: [TaskState; 6] = [
    TaskState::Draft,
    TaskState::InProgress,
    TaskState::InReview,
    TaskState::Paused,
    TaskState::Done,
    TaskState::Abandoned,
];

#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

#[fixture]
fn draft_task(clock: DefaultClock) -> Result<Task, TaskDomainError> {
    let issue_ref = IssueRef::from_parts("github", "owner/repo", 10)?;
    let metadata = ExternalIssueMetadata::new("State transition test")?;
    Ok(Task::new_from_issue(
        &ExternalIssue::new(issue_ref, metadata),
        &clock,
    ))
}

#[rstest]
#[case(TaskState::Draft, TaskState::Draft, false)]
#[case(TaskState::Draft, TaskState::InProgress, true)]
#[case(TaskState::Draft, TaskState::InReview, true)]
#[case(TaskState::Draft, TaskState::Paused, false)]
#[case(TaskState::Draft, TaskState::Done, false)]
#[case(TaskState::Draft, TaskState::Abandoned, true)]
#[case(TaskState::InProgress, TaskState::Draft, false)]
#[case(TaskState::InProgress, TaskState::InProgress, false)]
#[case(TaskState::InProgress, TaskState::InReview, true)]
#[case(TaskState::InProgress, TaskState::Paused, true)]
#[case(TaskState::InProgress, TaskState::Done, true)]
#[case(TaskState::InProgress, TaskState::Abandoned, true)]
#[case(TaskState::InReview, TaskState::Draft, false)]
#[case(TaskState::InReview, TaskState::InProgress, true)]
#[case(TaskState::InReview, TaskState::InReview, false)]
#[case(TaskState::InReview, TaskState::Paused, false)]
#[case(TaskState::InReview, TaskState::Done, true)]
#[case(TaskState::InReview, TaskState::Abandoned, true)]
#[case(TaskState::Paused, TaskState::Draft, false)]
#[case(TaskState::Paused, TaskState::InProgress, true)]
#[case(TaskState::Paused, TaskState::InReview, false)]
#[case(TaskState::Paused, TaskState::Paused, false)]
#[case(TaskState::Paused, TaskState::Done, false)]
#[case(TaskState::Paused, TaskState::Abandoned, true)]
#[case(TaskState::Done, TaskState::Draft, false)]
#[case(TaskState::Done, TaskState::InProgress, false)]
#[case(TaskState::Done, TaskState::InReview, false)]
#[case(TaskState::Done, TaskState::Paused, false)]
#[case(TaskState::Done, TaskState::Done, false)]
#[case(TaskState::Done, TaskState::Abandoned, false)]
#[case(TaskState::Abandoned, TaskState::Draft, false)]
#[case(TaskState::Abandoned, TaskState::InProgress, false)]
#[case(TaskState::Abandoned, TaskState::InReview, false)]
#[case(TaskState::Abandoned, TaskState::Paused, false)]
#[case(TaskState::Abandoned, TaskState::Done, false)]
#[case(TaskState::Abandoned, TaskState::Abandoned, false)]
fn can_transition_to_returns_expected(
    #[case] from: TaskState,
    #[case] to: TaskState,
    #[case] expected: bool,
) {
    assert_eq!(from.can_transition_to(to), expected);
}

#[rstest]
#[case(TaskState::Draft, false)]
#[case(TaskState::InProgress, false)]
#[case(TaskState::InReview, false)]
#[case(TaskState::Paused, false)]
#[case(TaskState::Done, true)]
#[case(TaskState::Abandoned, true)]
fn is_terminal_returns_expected(#[case] state: TaskState, #[case] expected: bool) {
    assert_eq!(state.is_terminal(), expected);
}

#[rstest]
fn transition_from_draft_to_in_progress_succeeds(
    clock: DefaultClock,
    draft_task: Result<Task, TaskDomainError>,
) -> eyre::Result<()> {
    let mut task = draft_task?;
    let original_updated_at = task.updated_at();

    task.transition_to(TaskState::InProgress, &clock)?;

    ensure!(task.state() == TaskState::InProgress);
    ensure!(task.updated_at() >= original_updated_at);
    Ok(())
}

#[rstest]
fn transition_from_draft_to_done_is_rejected(
    clock: DefaultClock,
    draft_task: Result<Task, TaskDomainError>,
) -> eyre::Result<()> {
    let mut task = draft_task?;
    let task_id = task.id();
    let original_state = task.state();

    let result = task.transition_to(TaskState::Done, &clock);
    let expected = Err(TaskDomainError::InvalidStateTransition {
        task_id,
        from: TaskState::Draft,
        to: TaskState::Done,
    });

    if result != expected {
        bail!("expected {expected:?}, got {result:?}");
    }
    ensure!(task.state() == original_state);
    Ok(())
}

#[rstest]
#[case(TaskState::Done)]
#[case(TaskState::Abandoned)]
fn terminal_state_rejects_all_transitions(
    #[case] terminal_state: TaskState,
    clock: DefaultClock,
    draft_task: Result<Task, TaskDomainError>,
) -> eyre::Result<()> {
    let mut task = draft_task?;

    if terminal_state == TaskState::Done {
        task.transition_to(TaskState::InProgress, &clock)?;
        task.transition_to(TaskState::Done, &clock)?;
    } else {
        task.transition_to(TaskState::Abandoned, &clock)?;
    }

    let task_id = task.id();
    for target_state in ALL_STATES {
        let result = task.transition_to(target_state, &clock);
        let expected = Err(TaskDomainError::InvalidStateTransition {
            task_id,
            from: terminal_state,
            to: target_state,
        });
        if result != expected {
            bail!("expected {expected:?}, got {result:?}");
        }
        ensure!(task.state() == terminal_state);
    }
    Ok(())
}

#[rstest]
fn associate_pull_request_allows_valid_transition_to_in_review(
    clock: DefaultClock,
    draft_task: Result<Task, TaskDomainError>,
) -> eyre::Result<()> {
    let mut task = draft_task?;
    task.transition_to(TaskState::InProgress, &clock)?;
    let original_updated_at = task.updated_at();
    let pr_ref = PullRequestRef::from_parts("github", "owner/repo", 42)?;

    task.associate_pull_request(pr_ref.clone(), &clock)?;

    ensure!(task.state() == TaskState::InReview);
    ensure!(task.pull_request_ref() == Some(&pr_ref));
    ensure!(task.updated_at() >= original_updated_at);
    Ok(())
}

#[rstest]
fn associate_pull_request_allows_existing_in_review_state(
    clock: DefaultClock,
    draft_task: Result<Task, TaskDomainError>,
) -> eyre::Result<()> {
    let mut task = draft_task?;
    task.transition_to(TaskState::InReview, &clock)?;
    let original_updated_at = task.updated_at();
    let pr_ref = PullRequestRef::from_parts("github", "owner/repo", 77)?;

    task.associate_pull_request(pr_ref.clone(), &clock)?;

    ensure!(task.state() == TaskState::InReview);
    ensure!(task.pull_request_ref() == Some(&pr_ref));
    ensure!(task.updated_at() >= original_updated_at);
    Ok(())
}

#[rstest]
#[case(TaskState::Done)]
#[case(TaskState::Abandoned)]
fn associate_pull_request_rejects_terminal_states_without_mutation(
    #[case] terminal_state: TaskState,
    clock: DefaultClock,
    draft_task: Result<Task, TaskDomainError>,
) -> eyre::Result<()> {
    let mut task = draft_task?;

    if terminal_state == TaskState::Done {
        task.transition_to(TaskState::InProgress, &clock)?;
        task.transition_to(TaskState::Done, &clock)?;
    } else {
        task.transition_to(TaskState::Abandoned, &clock)?;
    }

    let task_id = task.id();
    let original_updated_at = task.updated_at();
    let original_state = task.state();
    let pr_ref = PullRequestRef::from_parts("github", "owner/repo", 88)?;

    let result = task.associate_pull_request(pr_ref, &clock);
    let expected = Err(TaskDomainError::InvalidStateTransition {
        task_id,
        from: terminal_state,
        to: TaskState::InReview,
    });

    if result != expected {
        bail!("expected {expected:?}, got {result:?}");
    }
    ensure!(task.state() == original_state);
    ensure!(task.pull_request_ref().is_none());
    ensure!(task.updated_at() == original_updated_at);
    Ok(())
}

#[rstest]
fn associate_pull_request_rejects_duplicate_association(
    clock: DefaultClock,
    draft_task: Result<Task, TaskDomainError>,
) -> eyre::Result<()> {
    let mut task = draft_task?;
    task.transition_to(TaskState::InProgress, &clock)?;
    let first_pr_ref = PullRequestRef::from_parts("github", "owner/repo", 101)?;
    task.associate_pull_request(first_pr_ref.clone(), &clock)?;

    let original_updated_at = task.updated_at();
    let second_pr_ref = PullRequestRef::from_parts("github", "owner/repo", 102)?;
    let result = task.associate_pull_request(second_pr_ref, &clock);
    let expected = Err(TaskDomainError::PullRequestAlreadyAssociated(task.id()));

    if result != expected {
        bail!("expected {expected:?}, got {result:?}");
    }
    ensure!(task.state() == TaskState::InReview);
    ensure!(task.pull_request_ref() == Some(&first_pr_ref));
    ensure!(task.updated_at() == original_updated_at);
    Ok(())
}
