//! Unit tests for task state transition validation.

use crate::task::domain::{
    ExternalIssue, ExternalIssueMetadata, IssueRef, Task, TaskDomainError, TaskState,
};
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

fn create_draft_task(clock: &DefaultClock) -> Task {
    let issue_ref = IssueRef::from_parts("github", "owner/repo", 10).expect("valid issue ref");
    let metadata = ExternalIssueMetadata::new("State transition test").expect("valid title");
    Task::new_from_issue(&ExternalIssue::new(issue_ref, metadata), clock)
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
fn transition_from_draft_to_in_progress_succeeds(clock: DefaultClock) {
    let mut task = create_draft_task(&clock);
    let original_updated_at = task.updated_at();

    task.transition_to(TaskState::InProgress, &clock)
        .expect("transition should succeed");

    assert_eq!(task.state(), TaskState::InProgress);
    assert!(task.updated_at() >= original_updated_at);
}

#[rstest]
fn transition_from_draft_to_done_is_rejected(clock: DefaultClock) {
    let mut task = create_draft_task(&clock);
    let task_id = task.id();
    let original_state = task.state();

    let result = task.transition_to(TaskState::Done, &clock);

    assert_eq!(
        result,
        Err(TaskDomainError::InvalidStateTransition {
            task_id,
            from: TaskState::Draft,
            to: TaskState::Done,
        })
    );
    assert_eq!(task.state(), original_state);
}

#[rstest]
#[case(TaskState::Done)]
#[case(TaskState::Abandoned)]
fn terminal_state_rejects_all_transitions(#[case] terminal_state: TaskState, clock: DefaultClock) {
    let mut task = create_draft_task(&clock);

    if terminal_state == TaskState::Done {
        task.transition_to(TaskState::InProgress, &clock)
            .expect("draft to in_progress should succeed");
        task.transition_to(TaskState::Done, &clock)
            .expect("in_progress to done should succeed");
    } else {
        task.transition_to(TaskState::Abandoned, &clock)
            .expect("draft to abandoned should succeed");
    }

    let task_id = task.id();
    for target_state in ALL_STATES {
        let result = task.transition_to(target_state, &clock);
        assert_eq!(
            result,
            Err(TaskDomainError::InvalidStateTransition {
                task_id,
                from: terminal_state,
                to: target_state,
            })
        );
        assert_eq!(task.state(), terminal_state);
    }
}
