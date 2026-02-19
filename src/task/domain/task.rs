//! Task aggregate root and related task lifecycle types.

use super::{
    BranchRef, ExternalIssue, IssueRef, IssueSnapshot, ParseTaskStateError, PullRequestRef,
    TaskDomainError, TaskId,
};
use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Task lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task has been created but work has not started.
    Draft,
    /// Task is being implemented.
    InProgress,
    /// Task is awaiting review.
    InReview,
    /// Task work is temporarily paused.
    Paused,
    /// Task has been completed.
    Done,
    /// Task has been abandoned.
    Abandoned,
}

impl TaskState {
    /// Returns the canonical storage representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::InProgress => "in_progress",
            Self::InReview => "in_review",
            Self::Paused => "paused",
            Self::Done => "done",
            Self::Abandoned => "abandoned",
        }
    }

    /// Returns whether this state can transition to the target state.
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        match self {
            Self::Draft => matches!(target, Self::InProgress | Self::InReview | Self::Abandoned),
            Self::InProgress => matches!(
                target,
                Self::InReview | Self::Paused | Self::Done | Self::Abandoned
            ),
            Self::InReview => matches!(target, Self::InProgress | Self::Done | Self::Abandoned),
            Self::Paused => matches!(target, Self::InProgress | Self::Abandoned),
            Self::Done | Self::Abandoned => false,
        }
    }

    /// Returns whether this state is terminal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Done | Self::Abandoned)
    }
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for TaskState {
    type Error = ParseTaskStateError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "draft" => Ok(Self::Draft),
            "in_progress" => Ok(Self::InProgress),
            "in_review" => Ok(Self::InReview),
            "paused" => Ok(Self::Paused),
            "done" => Ok(Self::Done),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(ParseTaskStateError(value.to_owned())),
        }
    }
}

/// Origin information persisted with each task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskOrigin {
    /// Task created from an external issue reference.
    Issue {
        /// Canonical issue reference.
        issue_ref: IssueRef,
        /// Snapshot of issue metadata at task-creation time.
        metadata: IssueSnapshot,
    },
}

impl TaskOrigin {
    /// Returns the issue reference when the origin is issue-based.
    #[must_use]
    pub const fn issue_ref(&self) -> &IssueRef {
        match self {
            Self::Issue { issue_ref, .. } => issue_ref,
        }
    }
}

/// Task aggregate root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    id: TaskId,
    origin: TaskOrigin,
    branch_ref: Option<BranchRef>,
    pull_request_ref: Option<PullRequestRef>,
    state: TaskState,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Parameter object for reconstructing a persisted task aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedTaskData {
    /// Persisted task identifier.
    pub id: TaskId,
    /// Persisted origin metadata.
    pub origin: TaskOrigin,
    /// Persisted branch reference, if any.
    pub branch_ref: Option<BranchRef>,
    /// Persisted pull request reference, if any.
    pub pull_request_ref: Option<PullRequestRef>,
    /// Persisted lifecycle state.
    pub state: TaskState,
    /// Persisted creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Persisted latest lifecycle timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Task {
    /// Creates a new task from external issue data.
    #[must_use]
    pub fn new_from_issue(issue: &ExternalIssue, clock: &impl Clock) -> Self {
        let timestamp = clock.utc();
        let origin = TaskOrigin::Issue {
            issue_ref: issue.issue_ref().clone(),
            metadata: IssueSnapshot::from_external(issue.metadata().clone()),
        };

        Self {
            id: TaskId::new(),
            origin,
            branch_ref: None,
            pull_request_ref: None,
            state: TaskState::Draft,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    /// Reconstructs a task from persisted storage.
    #[must_use]
    pub fn from_persisted(data: PersistedTaskData) -> Self {
        Self {
            id: data.id,
            origin: data.origin,
            branch_ref: data.branch_ref,
            pull_request_ref: data.pull_request_ref,
            state: data.state,
            created_at: data.created_at,
            updated_at: data.updated_at,
        }
    }

    /// Returns the task identifier.
    #[must_use]
    pub const fn id(&self) -> TaskId {
        self.id
    }

    /// Returns the task origin.
    #[must_use]
    pub const fn origin(&self) -> &TaskOrigin {
        &self.origin
    }

    /// Returns the associated branch reference, if any.
    #[must_use]
    pub const fn branch_ref(&self) -> Option<&BranchRef> {
        self.branch_ref.as_ref()
    }

    /// Returns the associated pull request reference, if any.
    #[must_use]
    pub const fn pull_request_ref(&self) -> Option<&PullRequestRef> {
        self.pull_request_ref.as_ref()
    }

    /// Returns the task lifecycle state.
    #[must_use]
    pub const fn state(&self) -> TaskState {
        self.state
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns the latest lifecycle timestamp.
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Associates a branch with this task.
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::BranchAlreadyAssociated`] if a branch is
    /// already set.
    pub fn associate_branch(
        &mut self,
        branch_ref: BranchRef,
        clock: &impl Clock,
    ) -> Result<(), TaskDomainError> {
        associate_ref(
            &mut self.branch_ref,
            branch_ref,
            TaskDomainError::BranchAlreadyAssociated(self.id),
        )?;
        self.touch(clock);
        Ok(())
    }

    /// Associates a pull request with this task.
    ///
    /// Transitions the task state to [`TaskState::InReview`] as a side
    /// effect.
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::PullRequestAlreadyAssociated`] if a pull
    /// request is already set, or [`TaskDomainError::InvalidStateTransition`]
    /// when the task cannot transition to [`TaskState::InReview`] from its
    /// current state.
    pub fn associate_pull_request(
        &mut self,
        pr_ref: PullRequestRef,
        clock: &impl Clock,
    ) -> Result<(), TaskDomainError> {
        if self.pull_request_ref.is_some() {
            return Err(TaskDomainError::PullRequestAlreadyAssociated(self.id));
        }

        if self.state == TaskState::InReview {
            self.pull_request_ref = Some(pr_ref);
            self.touch(clock);
            return Ok(());
        }

        self.transition_to(TaskState::InReview, clock)?;
        self.pull_request_ref = Some(pr_ref);
        Ok(())
    }

    /// Transitions the task state when the transition is permitted.
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::InvalidStateTransition`] when the current
    /// task state cannot transition to `target`.
    pub fn transition_to(
        &mut self,
        target: TaskState,
        clock: &impl Clock,
    ) -> Result<(), TaskDomainError> {
        if !self.state.can_transition_to(target) {
            return Err(TaskDomainError::InvalidStateTransition {
                task_id: self.id,
                from: self.state,
                to: target,
            });
        }
        self.state = target;
        self.touch(clock);
        Ok(())
    }

    /// Updates the `updated_at` timestamp to the current clock time.
    fn touch(&mut self, clock: &impl Clock) {
        self.updated_at = clock.utc();
    }
}

/// Sets a reference field if empty, or returns the given error.
fn associate_ref<T>(
    field: &mut Option<T>,
    new_value: T,
    already_set_error: TaskDomainError,
) -> Result<(), TaskDomainError> {
    if field.is_some() {
        return Err(already_set_error);
    }
    *field = Some(new_value);
    Ok(())
}
