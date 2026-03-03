//! Action and hook execution status types.

use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt;
use thiserror::Error;

/// Action-level execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// The action completed successfully.
    Succeeded,
    /// The action failed.
    Failed,
    /// The action was skipped.
    Skipped,
}

impl ActionStatus {
    /// Returns the stable string representation.
    /// Example: `ActionStatus::Succeeded.as_str()` returns `"succeeded"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

impl fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing an action status fails.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown action status: {0}")]
pub struct ParseActionStatusError(pub String);

impl TryFrom<&str> for ActionStatus {
    type Error = ParseActionStatusError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            other => Err(ParseActionStatusError(other.to_owned())),
        }
    }
}

/// Hook execution status aggregated across actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookExecutionStatus {
    /// All actions succeeded or were skipped.
    Succeeded,
    /// All actions failed.
    Failed,
    /// A mix of successes and failures.
    PartialFailure,
}

impl HookExecutionStatus {
    /// Returns the stable string representation.
    /// Example: `HookExecutionStatus::Failed.as_str()` returns `"failed"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::PartialFailure => "partial_failure",
        }
    }

    /// Aggregates the overall status from action statuses.
    /// Example: `from_action_statuses([Succeeded, Failed])` returns `HookExecutionStatus::PartialFailure`.
    #[must_use]
    pub fn from_action_statuses<I>(statuses: I) -> Self
    where
        I: IntoIterator,
        I::Item: Borrow<ActionStatus>,
    {
        let mut saw_success = false;
        let mut saw_failure = false;

        for status in statuses {
            match *status.borrow() {
                ActionStatus::Succeeded => saw_success = true,
                ActionStatus::Failed => saw_failure = true,
                ActionStatus::Skipped => {}
            }
        }

        match (saw_success, saw_failure) {
            (true, true) => Self::PartialFailure,
            (false, true) => Self::Failed,
            _ => Self::Succeeded,
        }
    }
}

impl fmt::Display for HookExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing a hook execution status fails.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown hook execution status: {0}")]
pub struct ParseHookExecutionStatusError(pub String);

impl TryFrom<&str> for HookExecutionStatus {
    type Error = ParseHookExecutionStatusError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "partial_failure" => Ok(Self::PartialFailure),
            other => Err(ParseHookExecutionStatusError(other.to_owned())),
        }
    }
}
