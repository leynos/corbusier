//! Error types for task domain validation and parsing.

use thiserror::Error;

/// Errors returned while constructing domain task values.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TaskDomainError {
    /// The issue provider value is unsupported.
    #[error("unsupported issue provider: {0}")]
    InvalidIssueProvider(String),

    /// The repository name does not follow `owner/repo` format.
    #[error("invalid repository name '{0}', expected owner/repo")]
    InvalidRepository(String),

    /// The issue number is invalid.
    #[error("invalid issue number {0}, expected a positive integer")]
    InvalidIssueNumber(u64),

    /// The issue title is empty after trimming.
    #[error("issue title must not be empty")]
    EmptyIssueTitle,

    /// The branch name is invalid (empty, contains colons, or exceeds the
    /// length limit).
    #[error("invalid branch name: {0}")]
    InvalidBranchName(String),

    /// The pull request number is invalid.
    #[error("invalid pull request number {0}, expected a positive integer")]
    InvalidPullRequestNumber(u64),

    /// A branch is already associated with this task.
    #[error("task {0} already has an associated branch")]
    BranchAlreadyAssociated(super::TaskId),

    /// A pull request is already associated with this task.
    #[error("task {0} already has an associated pull request")]
    PullRequestAlreadyAssociated(super::TaskId),

    /// The canonical branch reference string could not be parsed.
    #[error("invalid branch reference format: {0}")]
    InvalidBranchRefFormat(String),

    /// The canonical pull request reference string could not be parsed.
    #[error("invalid pull request reference format: {0}")]
    InvalidPullRequestRefFormat(String),

    /// A canonical reference exceeds the `VARCHAR(255)` column limit.
    #[error("canonical reference exceeds 255-character storage limit: {0}")]
    CanonicalRefTooLong(String),
}

/// Error returned while parsing task states from persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown task state: {0}")]
pub struct ParseTaskStateError(pub String);
