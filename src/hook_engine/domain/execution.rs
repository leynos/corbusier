//! Hook execution results and log entries.

use super::{
    HookActionId, HookActionType, HookExecutionId, HookId, HookTriggerType, TriggerContextId,
};
use chrono::{DateTime, Utc};
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
    /// Returns the stable string representation. Example: `ActionStatus::Succeeded.as_str()` returns `"succeeded"`.
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
    /// Returns the stable string representation. Example: `HookExecutionStatus::Failed.as_str()` returns `"failed"`.
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
                ActionStatus::Succeeded => {
                    saw_success = true;
                }
                ActionStatus::Failed => {
                    saw_failure = true;
                }
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

/// Log severity level for hook execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookLogLevel {
    /// Informational log entry.
    Info,
    /// Warning log entry.
    Warning,
    /// Error log entry.
    Error,
}

/// Structured log entry emitted during hook execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookLogEntry {
    level: HookLogLevel,
    message: String,
    timestamp: DateTime<Utc>,
}

impl HookLogEntry {
    /// Creates a new log entry.
    /// Example: `HookLogEntry::new(HookLogLevel::Info, "ok", timestamp)` creates an info log entry.
    #[must_use]
    pub fn new(level: HookLogLevel, message: impl Into<String>, timestamp: DateTime<Utc>) -> Self {
        Self {
            level,
            message: message.into(),
            timestamp,
        }
    }

    /// Returns the log level. Example: `entry.level()` returns `HookLogLevel::Info`.
    #[must_use]
    pub const fn level(&self) -> HookLogLevel {
        self.level
    }

    /// Returns the log message. Example: `entry.message()` returns the log text.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the log timestamp. Example: `entry.timestamp()` returns the timestamp value.
    #[must_use]
    pub const fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

/// Result of executing a single hook action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    action_id: HookActionId,
    action_type: HookActionType,
    status: ActionStatus,
    output: serde_json::Value,
    log_entries: Vec<HookLogEntry>,
}

/// Input bundle for constructing an [`ActionResult`].
#[derive(Debug, Clone)]
pub struct ActionResultDetails {
    /// Identifier of the action that was executed.
    pub action_id: HookActionId,
    /// Type of action that was executed.
    pub action_type: HookActionType,
    /// Final status for the action execution.
    pub status: ActionStatus,
    /// Structured output payload produced by the action.
    pub output: serde_json::Value,
    /// Log entries captured while executing the action.
    pub log_entries: Vec<HookLogEntry>,
}

impl ActionResult {
    /// Creates a new action result.
    /// Example: `ActionResult::new(ActionResultDetails { action_id, action_type, status, output, log_entries })` builds a result.
    #[must_use]
    pub fn new(details: ActionResultDetails) -> Self {
        Self {
            action_id: details.action_id,
            action_type: details.action_type,
            status: details.status,
            output: details.output,
            log_entries: details.log_entries,
        }
    }

    /// Returns the action identifier. Example: `result.action_id()` returns the action identifier.
    #[must_use]
    pub const fn action_id(&self) -> &HookActionId {
        &self.action_id
    }

    /// Returns the action type.
    /// Example: `result.action_type()` returns the configured action type.
    #[must_use]
    pub const fn action_type(&self) -> &HookActionType {
        &self.action_type
    }

    /// Returns the action execution status.
    /// Example: `result.status()` returns `ActionStatus::Succeeded`.
    #[must_use]
    pub const fn status(&self) -> ActionStatus {
        self.status
    }

    /// Returns the action output payload.
    /// Example: `result.output()` returns the JSON payload.
    #[must_use]
    pub const fn output(&self) -> &serde_json::Value {
        &self.output
    }

    /// Returns the log entries for the action.
    /// Example: `result.log_entries()` returns action log entries.
    #[must_use]
    pub fn log_entries(&self) -> &[HookLogEntry] {
        &self.log_entries
    }
}

/// Result of executing an entire hook definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookExecutionResult {
    execution_id: HookExecutionId,
    hook_id: HookId,
    trigger_context_id: TriggerContextId,
    trigger_type: HookTriggerType,
    predicate_data: serde_json::Value,
    action_results: Vec<ActionResult>,
    status: HookExecutionStatus,
    executed_at: DateTime<Utc>,
}

/// Input bundle for constructing a [`HookExecutionResult`].
#[derive(Debug, Clone)]
pub struct HookExecutionInput {
    /// Identifier of the executed hook definition.
    pub hook_id: HookId,
    /// Identifier of the trigger context that initiated execution.
    pub trigger_context_id: TriggerContextId,
    /// Trigger type that selected this hook definition.
    pub trigger_type: HookTriggerType,
    /// Predicate payload captured at execution time.
    pub predicate_data: serde_json::Value,
    /// Action-level execution outputs for this hook run.
    pub action_results: Vec<ActionResult>,
    /// Execution timestamp.
    pub executed_at: DateTime<Utc>,
}

/// Data required to reconstruct a [`HookExecutionResult`] from persisted storage.
#[derive(Debug)]
pub struct HookExecutionPersisted {
    /// Identifier of the persisted execution record.
    pub execution_id: HookExecutionId,
    /// Identifier of the executed hook definition.
    pub hook_id: HookId,
    /// Identifier of the trigger context that initiated execution.
    pub trigger_context_id: TriggerContextId,
    /// Trigger type that selected this hook definition.
    pub trigger_type: HookTriggerType,
    /// Predicate payload captured at execution time.
    pub predicate_data: serde_json::Value,
    /// Action-level execution outputs for this hook run.
    pub action_results: Vec<ActionResult>,
    /// Aggregated execution status.
    pub status: HookExecutionStatus,
    /// Execution timestamp.
    pub executed_at: DateTime<Utc>,
}

impl HookExecutionResult {
    /// Creates a new hook execution result.
    /// Example: `HookExecutionResult::new(HookExecutionInput { hook_id, trigger_context_id, trigger_type, predicate_data, action_results, executed_at })` computes the status.
    #[must_use]
    pub fn new(input: HookExecutionInput) -> Self {
        let status = HookExecutionStatus::from_action_statuses(
            input.action_results.iter().map(ActionResult::status),
        );
        Self {
            execution_id: HookExecutionId::new(),
            hook_id: input.hook_id,
            trigger_context_id: input.trigger_context_id,
            trigger_type: input.trigger_type,
            predicate_data: input.predicate_data,
            action_results: input.action_results,
            status,
            executed_at: input.executed_at,
        }
    }

    /// Creates a hook execution result from persisted fields.
    /// Example: `from_persisted(HookExecutionPersisted { .. })` restores stored records.
    #[must_use]
    pub fn from_persisted(persisted: HookExecutionPersisted) -> Self {
        Self {
            execution_id: persisted.execution_id,
            hook_id: persisted.hook_id,
            trigger_context_id: persisted.trigger_context_id,
            trigger_type: persisted.trigger_type,
            predicate_data: persisted.predicate_data,
            action_results: persisted.action_results,
            status: persisted.status,
            executed_at: persisted.executed_at,
        }
    }

    /// Returns the execution identifier.
    /// Example: `result.execution_id()` returns the execution ID.
    #[must_use]
    pub const fn execution_id(&self) -> HookExecutionId {
        self.execution_id
    }

    /// Returns the hook identifier.
    /// Example: `result.hook_id()` returns the hook ID.
    #[must_use]
    pub const fn hook_id(&self) -> &HookId {
        &self.hook_id
    }

    /// Returns the trigger context identifier.
    /// Example: `result.trigger_context_id()` returns the context ID.
    #[must_use]
    pub const fn trigger_context_id(&self) -> TriggerContextId {
        self.trigger_context_id
    }

    /// Returns the trigger type.
    /// Example: `result.trigger_type()` returns the trigger type.
    #[must_use]
    pub const fn trigger_type(&self) -> HookTriggerType {
        self.trigger_type
    }

    /// Returns the predicate data payload.
    /// Example: `result.predicate_data()` returns the predicate JSON.
    #[must_use]
    pub const fn predicate_data(&self) -> &serde_json::Value {
        &self.predicate_data
    }

    /// Returns the action results.
    /// Example: `result.action_results()` returns the action outputs.
    #[must_use]
    pub fn action_results(&self) -> &[ActionResult] {
        &self.action_results
    }

    /// Returns the overall hook execution status.
    /// Example: `result.status()` returns the aggregated status.
    #[must_use]
    pub const fn status(&self) -> HookExecutionStatus {
        self.status
    }

    /// Returns the execution timestamp.
    /// Example: `result.executed_at()` returns the execution time.
    #[must_use]
    pub const fn executed_at(&self) -> DateTime<Utc> {
        self.executed_at
    }
}
