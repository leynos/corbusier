//! Issue-origin value objects for task creation.

use super::{IssueNumber, RepositoryFullName, TaskDomainError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported external issue providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IssueProvider {
    /// GitHub issues.
    #[serde(rename = "github")]
    GitHub,
    /// GitLab issues.
    #[serde(rename = "gitlab")]
    GitLab,
}

impl IssueProvider {
    /// Returns provider name in canonical storage format.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::GitLab => "gitlab",
        }
    }
}

impl TryFrom<&str> for IssueProvider {
    type Error = TaskDomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "github" => Ok(Self::GitHub),
            "gitlab" => Ok(Self::GitLab),
            _ => Err(TaskDomainError::InvalidIssueProvider(value.to_owned())),
        }
    }
}

impl fmt::Display for IssueProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Canonical external issue reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IssueRef {
    provider: IssueProvider,
    repository: RepositoryFullName,
    issue_number: IssueNumber,
}

impl IssueRef {
    /// Creates an issue reference from validated components.
    #[must_use]
    pub const fn new(
        provider: IssueProvider,
        repository: RepositoryFullName,
        issue_number: IssueNumber,
    ) -> Self {
        Self {
            provider,
            repository,
            issue_number,
        }
    }

    /// Creates an issue reference from raw external values.
    ///
    /// # Errors
    ///
    /// Returns a [`TaskDomainError`] when any component is invalid.
    pub fn from_parts(
        provider: &str,
        repository: &str,
        issue_number: u64,
    ) -> Result<Self, TaskDomainError> {
        Ok(Self::new(
            IssueProvider::try_from(provider)?,
            RepositoryFullName::new(repository)?,
            IssueNumber::new(issue_number)?,
        ))
    }

    /// Returns the issue provider.
    #[must_use]
    pub const fn provider(&self) -> IssueProvider {
        self.provider
    }

    /// Returns the repository identifier.
    #[must_use]
    pub const fn repository(&self) -> &RepositoryFullName {
        &self.repository
    }

    /// Returns the issue number.
    #[must_use]
    pub const fn issue_number(&self) -> IssueNumber {
        self.issue_number
    }
}

impl fmt::Display for IssueRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/#{}",
            self.provider, self.repository, self.issue_number
        )
    }
}

/// External issue metadata as received from VCS providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalIssueMetadata {
    title: String,
    description: Option<String>,
    labels: Vec<String>,
    assignees: Vec<String>,
    milestone: Option<String>,
}

impl ExternalIssueMetadata {
    /// Creates external issue metadata with required title.
    ///
    /// # Errors
    ///
    /// Returns [`TaskDomainError::EmptyIssueTitle`] if the title is empty.
    pub fn new(title: impl Into<String>) -> Result<Self, TaskDomainError> {
        let raw_title = title.into();
        let normalized_title = raw_title.trim();
        if normalized_title.is_empty() {
            return Err(TaskDomainError::EmptyIssueTitle);
        }

        Ok(Self {
            title: normalized_title.to_owned(),
            description: None,
            labels: Vec::new(),
            assignees: Vec::new(),
            milestone: None,
        })
    }

    /// Sets issue description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        let value = description.into();
        let normalized = value.trim();
        self.description = (!normalized.is_empty()).then_some(normalized.to_owned());
        self
    }

    /// Sets issue labels.
    #[must_use]
    pub fn with_labels(mut self, labels: impl IntoIterator<Item = String>) -> Self {
        self.labels = labels
            .into_iter()
            .map(|label| label.trim().to_owned())
            .filter(|label| !label.is_empty())
            .collect();
        self
    }

    /// Sets issue assignees.
    #[must_use]
    pub fn with_assignees(mut self, assignees: impl IntoIterator<Item = String>) -> Self {
        self.assignees = assignees
            .into_iter()
            .map(|assignee| assignee.trim().to_owned())
            .filter(|assignee| !assignee.is_empty())
            .collect();
        self
    }

    /// Sets issue milestone.
    #[must_use]
    pub fn with_milestone(mut self, milestone: impl Into<String>) -> Self {
        let value = milestone.into();
        let normalized = value.trim();
        self.milestone = (!normalized.is_empty()).then_some(normalized.to_owned());
        self
    }

    /// Returns the issue title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the issue description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns issue labels.
    #[must_use]
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Returns issue assignees.
    #[must_use]
    pub fn assignees(&self) -> &[String] {
        &self.assignees
    }

    /// Returns issue milestone.
    #[must_use]
    pub fn milestone(&self) -> Option<&str> {
        self.milestone.as_deref()
    }
}

/// Internal snapshot of issue metadata persisted with task origin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueSnapshot {
    /// Issue title as persisted at creation time.
    pub title: String,
    /// Issue description as persisted at creation time.
    pub description: Option<String>,
    /// Issue labels as persisted at creation time.
    pub labels: Vec<String>,
    /// Issue assignees as persisted at creation time.
    pub assignees: Vec<String>,
    /// Issue milestone as persisted at creation time.
    pub milestone: Option<String>,
}

impl IssueSnapshot {
    /// Creates an internal snapshot from external metadata.
    #[must_use]
    pub fn from_external(metadata: ExternalIssueMetadata) -> Self {
        Self {
            title: metadata.title,
            description: metadata.description,
            labels: metadata.labels,
            assignees: metadata.assignees,
            milestone: metadata.milestone,
        }
    }

    /// Returns the persisted issue title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// External issue payload used to create a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalIssue {
    issue_ref: IssueRef,
    metadata: ExternalIssueMetadata,
}

impl ExternalIssue {
    /// Creates an external issue payload from validated values.
    #[must_use]
    pub const fn new(issue_ref: IssueRef, metadata: ExternalIssueMetadata) -> Self {
        Self {
            issue_ref,
            metadata,
        }
    }

    /// Returns the issue reference.
    #[must_use]
    pub const fn issue_ref(&self) -> &IssueRef {
        &self.issue_ref
    }

    /// Returns the issue metadata.
    #[must_use]
    pub const fn metadata(&self) -> &ExternalIssueMetadata {
        &self.metadata
    }
}
