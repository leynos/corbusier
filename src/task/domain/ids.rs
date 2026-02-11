//! Identifier and validated scalar types for the task domain.

use super::TaskDomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an internal task record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Creates a new random task identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a task identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the wrapped UUID.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for TaskId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Positive issue number from an external tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IssueNumber(u64);

impl IssueNumber {
    /// Largest issue number representable in the current `PostgreSQL` schema.
    const MAX_PERSISTED_VALUE: u64 = i64::MAX as u64;

    /// Creates a validated issue number.
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::InvalidIssueNumber`] when the value is zero
    /// or exceeds the schema-backed maximum (`i64::MAX`).
    pub const fn new(value: u64) -> Result<Self, TaskDomainError> {
        if value == 0 || value > Self::MAX_PERSISTED_VALUE {
            return Err(TaskDomainError::InvalidIssueNumber(value));
        }
        Ok(Self(value))
    }

    /// Returns the underlying numeric value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for IssueNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Normalized external repository identifier in `owner/repo` format.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepositoryFullName(String);

impl RepositoryFullName {
    /// Creates a validated repository name.
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::InvalidRepository`] if the value does not
    /// contain exactly one slash-delimited owner and repository segment.
    pub fn new(value: impl Into<String>) -> Result<Self, TaskDomainError> {
        let raw = value.into();
        let normalized = raw.trim();
        let mut segments = normalized.split('/');
        let owner = segments.next().unwrap_or_default();
        let repo = segments.next().unwrap_or_default();
        let has_more_segments = segments.next().is_some();
        let is_valid = !owner.is_empty()
            && !repo.is_empty()
            && !has_more_segments
            && !normalized.chars().any(char::is_whitespace);

        if !is_valid {
            return Err(TaskDomainError::InvalidRepository(raw));
        }

        Ok(Self(normalized.to_owned()))
    }

    /// Returns the repository name as `str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for RepositoryFullName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for RepositoryFullName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
