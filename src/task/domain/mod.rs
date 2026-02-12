//! Domain model for task lifecycle management.
//!
//! The task domain models issue-origin task creation, branch and pull request
//! association, and lookup while keeping all infrastructure concerns outside
//! of the domain boundary.

mod branch;
mod error;
mod ids;
mod issue;
mod pull_request;
mod task;

pub use branch::{BranchName, BranchRef};
pub use error::{ParseTaskStateError, TaskDomainError};
pub use ids::{IssueNumber, RepositoryFullName, TaskId};
pub use issue::{ExternalIssue, ExternalIssueMetadata, IssueProvider, IssueRef, IssueSnapshot};
pub use pull_request::{PullRequestNumber, PullRequestRef};
pub use task::{PersistedTaskData, Task, TaskOrigin, TaskState};

/// Type alias exposing [`IssueProvider`] under a VCS-agnostic name for use
/// in branch and pull request contexts.
pub type VcsProvider = IssueProvider;
