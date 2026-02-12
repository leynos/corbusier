//! Pull-request-reference value objects for task-PR association.

use super::{IssueProvider, RepositoryFullName, TaskDomainError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Positive pull request number from an external tracker.
///
/// Validated identically to [`super::IssueNumber`]: must be positive and
/// representable by `PostgreSQL` `BIGINT` (`<= i64::MAX`).
///
/// # Examples
///
///     use corbusier::task::domain::PullRequestNumber;
///
///     let pr_num = PullRequestNumber::new(42).expect("valid");
///     assert_eq!(pr_num.value(), 42);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PullRequestNumber(u64);

impl PullRequestNumber {
    /// Largest pull request number representable in the current schema.
    const MAX_PERSISTED_VALUE: u64 = i64::MAX as u64;

    /// Creates a validated pull request number.
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::InvalidPullRequestNumber`] when the value
    /// is zero or exceeds the schema-backed maximum (`i64::MAX`).
    pub const fn new(value: u64) -> Result<Self, TaskDomainError> {
        if value == 0 || value > Self::MAX_PERSISTED_VALUE {
            return Err(TaskDomainError::InvalidPullRequestNumber(value));
        }
        Ok(Self(value))
    }

    /// Returns the underlying numeric value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl fmt::Display for PullRequestNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Canonical pull request reference scoped to a VCS provider and repository.
///
/// Persisted as `"provider:owner/repo:42"` in the `pull_request_ref` column.
///
/// # Examples
///
///     use corbusier::task::domain::PullRequestRef;
///
///     let pr = PullRequestRef::from_parts("github", "owner/repo", 42)
///         .expect("valid PR ref");
///     assert_eq!(pr.to_string(), "github:owner/repo:42");
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PullRequestRef {
    provider: IssueProvider,
    repository: RepositoryFullName,
    pull_request_number: PullRequestNumber,
}

impl PullRequestRef {
    /// Creates a pull request reference from validated components.
    #[must_use]
    pub const fn new(
        provider: IssueProvider,
        repository: RepositoryFullName,
        pull_request_number: PullRequestNumber,
    ) -> Self {
        Self {
            provider,
            repository,
            pull_request_number,
        }
    }

    /// Creates a pull request reference from raw external values.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskDomainError`] when any component is invalid.
    pub fn from_parts(
        provider: &str,
        repository: &str,
        pull_request_number: u64,
    ) -> Result<Self, TaskDomainError> {
        Ok(Self::new(
            IssueProvider::try_from(provider)?,
            RepositoryFullName::new(repository)?,
            PullRequestNumber::new(pull_request_number)?,
        ))
    }

    /// Produces the canonical storage representation.
    #[must_use]
    pub fn to_canonical(&self) -> String {
        format!(
            "{}:{}:{}",
            self.provider, self.repository, self.pull_request_number
        )
    }

    /// Parses a canonical string (`"provider:owner/repo:42"`) back into a
    /// [`PullRequestRef`].
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::InvalidPullRequestRefFormat`] when the
    /// input does not match the expected format.
    pub fn parse_canonical(s: &str) -> Result<Self, TaskDomainError> {
        let (provider_str, rest) = s
            .split_once(':')
            .ok_or_else(|| TaskDomainError::InvalidPullRequestRefFormat(s.to_owned()))?;
        let (repository_str, number_str) = rest
            .split_once(':')
            .ok_or_else(|| TaskDomainError::InvalidPullRequestRefFormat(s.to_owned()))?;

        let number: u64 = number_str
            .parse()
            .map_err(|_| TaskDomainError::InvalidPullRequestRefFormat(s.to_owned()))?;

        Self::from_parts(provider_str, repository_str, number)
            .map_err(|_| TaskDomainError::InvalidPullRequestRefFormat(s.to_owned()))
    }

    /// Returns the VCS provider.
    #[must_use]
    pub const fn provider(&self) -> IssueProvider {
        self.provider
    }

    /// Returns the repository identifier.
    #[must_use]
    pub const fn repository(&self) -> &RepositoryFullName {
        &self.repository
    }

    /// Returns the pull request number.
    #[must_use]
    pub const fn pull_request_number(&self) -> PullRequestNumber {
        self.pull_request_number
    }
}

impl fmt::Display for PullRequestRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

impl TryFrom<&str> for PullRequestRef {
    type Error = TaskDomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse_canonical(value)
    }
}
