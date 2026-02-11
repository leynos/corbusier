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
}

/// Error returned while parsing task states from persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown task state: {0}")]
pub struct ParseTaskStateError(pub String);
