//! Branch-reference value objects for task-branch association.

use super::{IssueProvider, RepositoryFullName, TaskDomainError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum length for a validated branch name.
///
/// Set to 200 to leave room for the `provider:owner/repo:` prefix within the
/// 255-character `branch_ref` column.
const MAX_BRANCH_NAME_LENGTH: usize = 200;

/// Maximum length for a canonical reference stored in a `VARCHAR(255)` column.
const MAX_CANONICAL_REF_LENGTH: usize = 255;

/// Validated Git branch name.
///
/// Branch names must be non-empty after trimming, must not contain colon
/// characters (reserved as the canonical-format delimiter), and must not
/// exceed `MAX_BRANCH_NAME_LENGTH` characters.
///
/// # Examples
///
///     use corbusier::task::domain::BranchName;
///
///     let name = BranchName::new("feature/my-branch").expect("valid");
///     assert_eq!(name.as_str(), "feature/my-branch");
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BranchName(String);

impl BranchName {
    /// Creates a validated branch name.
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::InvalidBranchName`] when the value is empty,
    /// contains a colon, or exceeds the length limit.
    pub fn new(value: impl Into<String>) -> Result<Self, TaskDomainError> {
        let raw = value.into();
        let normalized = raw.trim();

        if Self::is_invalid_branch_name(normalized) {
            return Err(TaskDomainError::InvalidBranchName(raw));
        }

        Ok(Self(normalized.to_owned()))
    }

    /// Validates branch name constraints.
    fn is_invalid_branch_name(name: &str) -> bool {
        let is_empty = name.is_empty();
        let contains_forbidden_char = name.contains(':');
        let exceeds_length_limit = name.len() > MAX_BRANCH_NAME_LENGTH;

        is_empty || contains_forbidden_char || exceeds_length_limit
    }

    /// Returns the branch name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for BranchName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for BranchName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Canonical branch reference scoped to a VCS provider and repository.
///
/// Persisted as `"provider:owner/repo:branch-name"` in the `branch_ref`
/// column. Colons are forbidden in Git ref names by `git-check-ref-format`,
/// making the format unambiguous.
///
/// # Examples
///
///     use corbusier::task::domain::BranchRef;
///
///     let branch = BranchRef::from_parts("github", "owner/repo", "main")
///         .expect("valid branch ref");
///     assert_eq!(branch.to_string(), "github:owner/repo:main");
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchRef {
    provider: IssueProvider,
    repository: RepositoryFullName,
    branch_name: BranchName,
}

impl BranchRef {
    /// Creates a branch reference from validated components.
    ///
    /// Restricted to crate scope to ensure all external construction goes
    /// through [`Self::from_parts`], which enforces the canonical-length
    /// invariant.
    #[must_use]
    pub(crate) const fn new(
        provider: IssueProvider,
        repository: RepositoryFullName,
        branch_name: BranchName,
    ) -> Self {
        Self {
            provider,
            repository,
            branch_name,
        }
    }

    /// Creates a branch reference from raw external values.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskDomainError`] when any component is invalid or the
    /// canonical representation exceeds the `VARCHAR(255)` storage limit.
    pub fn from_parts(
        provider: &str,
        repository: &str,
        branch_name: &str,
    ) -> Result<Self, TaskDomainError> {
        let branch_ref = Self::new(
            IssueProvider::try_from(provider)?,
            RepositoryFullName::new(repository)?,
            BranchName::new(branch_name)?,
        );
        let canonical = branch_ref.to_canonical();
        if canonical.len() > MAX_CANONICAL_REF_LENGTH {
            return Err(TaskDomainError::CanonicalRefTooLong(canonical));
        }
        Ok(branch_ref)
    }

    /// Produces the canonical storage representation.
    #[must_use]
    pub fn to_canonical(&self) -> String {
        format!("{}:{}:{}", self.provider, self.repository, self.branch_name)
    }

    /// Parses a canonical string (`"provider:owner/repo:branch-name"`) back
    /// into a [`BranchRef`].
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::InvalidBranchRefFormat`] when the input
    /// does not match the expected format.
    pub fn parse_canonical(s: &str) -> Result<Self, TaskDomainError> {
        // Split on first colon to get provider, then split remainder on
        // second colon to get repository and branch name.
        let (provider_str, rest) = s
            .split_once(':')
            .ok_or_else(|| TaskDomainError::InvalidBranchRefFormat(s.to_owned()))?;
        let (repository_str, branch_str) = rest
            .split_once(':')
            .ok_or_else(|| TaskDomainError::InvalidBranchRefFormat(s.to_owned()))?;

        Self::from_parts(provider_str, repository_str, branch_str)
            .map_err(|_| TaskDomainError::InvalidBranchRefFormat(s.to_owned()))
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

    /// Returns the branch name.
    #[must_use]
    pub const fn branch_name(&self) -> &BranchName {
        &self.branch_name
    }
}

impl fmt::Display for BranchRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

impl TryFrom<&str> for BranchRef {
    type Error = TaskDomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse_canonical(value)
    }
}
