//! Hook trigger types, execution scope, and trigger context metadata.

use super::TriggerContextId;
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use crate::tool_registry::domain::ToolExecutionScope;
use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Canonical hook trigger types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookTriggerType {
    /// A new conversation turn is starting.
    TurnStart,
    /// A conversation turn has ended.
    TurnEnd,
    /// A tool call is about to execute.
    PreToolUse,
    /// A tool call has completed.
    PostToolUse,
    /// A commit is about to be created.
    PreCommit,
    /// A commit has been created.
    PostCommit,
    /// A merge is about to occur.
    PreMerge,
    /// A merge has completed.
    PostMerge,
    /// A pull is about to occur.
    PrePull,
    /// A pull has completed.
    PostPull,
    /// A push is about to occur.
    PrePush,
    /// A push has completed.
    PostPush,
    /// A deployment is about to start.
    PreDeploy,
    /// A deployment has completed.
    PostDeploy,
}

impl HookTriggerType {
    /// Returns the stable string representation for persistence.
    ///
    /// Example: `HookTriggerType::PreCommit.as_str()` returns `"pre_commit"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TurnStart => "turn_start",
            Self::TurnEnd => "turn_end",
            Self::PreToolUse => "pre_tool_use",
            Self::PostToolUse => "post_tool_use",
            Self::PreCommit => "pre_commit",
            Self::PostCommit => "post_commit",
            Self::PreMerge => "pre_merge",
            Self::PostMerge => "post_merge",
            Self::PrePull => "pre_pull",
            Self::PostPull => "post_pull",
            Self::PrePush => "pre_push",
            Self::PostPush => "post_push",
            Self::PreDeploy => "pre_deploy",
            Self::PostDeploy => "post_deploy",
        }
    }

    /// Returns all supported trigger types.
    ///
    /// Example: `HookTriggerType::all()` includes `HookTriggerType::TurnStart`.
    #[must_use]
    pub const fn all() -> [Self; 14] {
        [
            Self::TurnStart,
            Self::TurnEnd,
            Self::PreToolUse,
            Self::PostToolUse,
            Self::PreCommit,
            Self::PostCommit,
            Self::PreMerge,
            Self::PostMerge,
            Self::PrePull,
            Self::PostPull,
            Self::PrePush,
            Self::PostPush,
            Self::PreDeploy,
            Self::PostDeploy,
        ]
    }
}

impl fmt::Display for HookTriggerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Error returned when parsing a trigger type fails.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown hook trigger type: {0}")]
pub struct ParseHookTriggerTypeError(pub String);

impl TryFrom<&str> for HookTriggerType {
    type Error = ParseHookTriggerTypeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "turn_start" => Ok(Self::TurnStart),
            "turn_end" => Ok(Self::TurnEnd),
            "pre_tool_use" => Ok(Self::PreToolUse),
            "post_tool_use" => Ok(Self::PostToolUse),
            "pre_commit" => Ok(Self::PreCommit),
            "post_commit" => Ok(Self::PostCommit),
            "pre_merge" => Ok(Self::PreMerge),
            "post_merge" => Ok(Self::PostMerge),
            "pre_pull" => Ok(Self::PrePull),
            "post_pull" => Ok(Self::PostPull),
            "pre_push" => Ok(Self::PrePush),
            "post_push" => Ok(Self::PostPush),
            "pre_deploy" => Ok(Self::PreDeploy),
            "post_deploy" => Ok(Self::PostDeploy),
            other => Err(ParseHookTriggerTypeError(other.to_owned())),
        }
    }
}

/// Metadata describing a trigger invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookTriggerContext {
    id: TriggerContextId,
    trigger_type: HookTriggerType,
    execution_scope: HookExecutionScope,
    occurred_at: DateTime<Utc>,
}

/// Workflow correlation scope carried alongside a trigger invocation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookExecutionScope {
    task_id: Option<TaskId>,
    conversation_id: Option<ConversationId>,
    metadata: serde_json::Value,
}

impl HookExecutionScope {
    /// Creates an empty execution scope with no task, conversation, or metadata.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            task_id: None,
            conversation_id: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Associates a task with the execution scope.
    #[must_use]
    pub const fn with_task_id(mut self, task_id: TaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Associates a conversation with the execution scope.
    #[must_use]
    pub const fn with_conversation_id(mut self, conversation_id: ConversationId) -> Self {
        self.conversation_id = Some(conversation_id);
        self
    }

    /// Attaches non-indexed execution metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Returns the correlated task identifier, if any.
    #[must_use]
    pub const fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    /// Returns the correlated conversation identifier, if any.
    #[must_use]
    pub const fn conversation_id(&self) -> Option<ConversationId> {
        self.conversation_id
    }

    /// Returns the non-indexed execution metadata payload.
    #[must_use]
    pub const fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }
}

impl Default for HookExecutionScope {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&ToolExecutionScope> for HookExecutionScope {
    fn from(scope: &ToolExecutionScope) -> Self {
        let mut hook_scope = Self::default();
        if let Some(task_id) = scope.task_id() {
            hook_scope = hook_scope.with_task_id(task_id);
        }
        if let Some(conversation_id) = scope.conversation_id() {
            hook_scope = hook_scope.with_conversation_id(conversation_id);
        }
        hook_scope
    }
}

impl HookTriggerContext {
    /// Creates a trigger context with the current time from a clock.
    ///
    /// Example: `HookTriggerContext::new(HookTriggerType::TurnStart, &clock)`
    /// stamps the context with the current time.
    #[must_use]
    pub fn new(trigger_type: HookTriggerType, clock: &impl Clock) -> Self {
        Self::with_execution_scope(trigger_type, HookExecutionScope::default(), clock)
    }

    /// Creates a trigger context with metadata and the current time.
    ///
    /// Example: `with_metadata(trigger, json!({\"tool\": \"rg\"}), &clock)`
    /// stores metadata alongside the trigger.
    #[must_use]
    pub fn with_metadata(
        trigger_type: HookTriggerType,
        metadata: serde_json::Value,
        clock: &impl Clock,
    ) -> Self {
        Self::with_execution_scope(
            trigger_type,
            HookExecutionScope::default().with_metadata(metadata),
            clock,
        )
    }

    /// Creates a trigger context with a typed execution scope and the current
    /// time.
    #[must_use]
    pub fn with_execution_scope(
        trigger_type: HookTriggerType,
        execution_scope: HookExecutionScope,
        clock: &impl Clock,
    ) -> Self {
        Self::new_with_timestamp(trigger_type, execution_scope, clock.utc())
    }

    /// Creates a trigger context with an explicit timestamp.
    ///
    /// Example: `new_with_timestamp(trigger, HookExecutionScope::default(),
    /// timestamp)` uses the supplied time.
    #[must_use]
    pub fn new_with_timestamp(
        trigger_type: HookTriggerType,
        execution_scope: HookExecutionScope,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: TriggerContextId::new(),
            trigger_type,
            execution_scope,
            occurred_at,
        }
    }

    /// Returns the trigger context identifier.
    ///
    /// Example: `context.id()` returns the trigger context ID.
    #[must_use]
    pub const fn id(&self) -> TriggerContextId {
        self.id
    }

    /// Returns the trigger type.
    ///
    /// Example: `context.trigger_type()` returns the trigger type.
    #[must_use]
    pub const fn trigger_type(&self) -> HookTriggerType {
        self.trigger_type
    }

    /// Returns the trigger metadata payload.
    ///
    /// Example: `context.metadata()` returns the metadata JSON.
    #[must_use]
    pub const fn metadata(&self) -> &serde_json::Value {
        self.execution_scope.metadata()
    }

    /// Returns the typed execution scope associated with the trigger.
    #[must_use]
    pub const fn execution_scope(&self) -> &HookExecutionScope {
        &self.execution_scope
    }

    /// Returns the trigger occurrence time.
    ///
    /// Example: `context.occurred_at()` returns the timestamp.
    #[must_use]
    pub const fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

impl From<crate::tool_registry::domain::ToolExecutionScope> for HookExecutionScope {
    fn from(src: crate::tool_registry::domain::ToolExecutionScope) -> Self {
        let mut scope = Self::new();
        if let Some(task_id) = src.task_id() {
            scope = scope.with_task_id(task_id);
        }
        if let Some(conversation_id) = src.conversation_id() {
            scope = scope.with_conversation_id(conversation_id);
        }
        scope.with_metadata(src.metadata().clone())
    }
}
