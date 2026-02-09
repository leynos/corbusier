//! Domain model for task lifecycle management.
//!
//! The task domain models issue-origin task creation and lookup while keeping
//! all infrastructure concerns outside of the domain boundary.

mod error;
mod ids;
mod issue;
mod task;

pub use error::{ParseTaskStateError, TaskDomainError};
pub use ids::{IssueNumber, RepositoryFullName, TaskId};
pub use issue::{ExternalIssue, ExternalIssueMetadata, IssueProvider, IssueRef, IssueSnapshot};
pub use task::{PersistedTaskData, Task, TaskOrigin, TaskState};
