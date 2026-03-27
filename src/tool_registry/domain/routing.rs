//! Tool call routing domain types.
//!
//! These types model the lifecycle of a routed tool call: the inbound
//! request, the outcome, and the completed result with timing metadata.

use super::McpServerId;
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::time::Duration;
use uuid::Uuid;

/// Unique identifier for a tool call invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ToolCallId(Uuid);

impl ToolCallId {
    /// Creates a new random tool call identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a tool call identifier from an existing UUID.
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

impl Default for ToolCallId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ToolCallId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Inbound request to invoke a tool by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallRequest {
    call_id: ToolCallId,
    tool_name: String,
    parameters: Value,
    execution_scope: ToolExecutionScope,
    initiated_at: DateTime<Utc>,
}

/// Workflow correlation scope for a routed tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecutionScope {
    task_id: Option<TaskId>,
    conversation_id: Option<ConversationId>,
    metadata: Value,
}

impl ToolExecutionScope {
    /// Creates an empty execution scope.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            task_id: None,
            conversation_id: None,
            metadata: Value::Null,
        }
    }

    /// Associates a task with the tool call.
    #[must_use]
    pub const fn with_task_id(mut self, task_id: TaskId) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Associates a conversation with the tool call.
    #[must_use]
    pub const fn with_conversation_id(mut self, conversation_id: ConversationId) -> Self {
        self.conversation_id = Some(conversation_id);
        self
    }

    /// Attaches additional non-indexed execution metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Returns the associated task identifier, if any.
    #[must_use]
    pub const fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    /// Returns the associated conversation identifier, if any.
    #[must_use]
    pub const fn conversation_id(&self) -> Option<ConversationId> {
        self.conversation_id
    }

    /// Returns non-indexed metadata for the tool execution.
    #[must_use]
    pub const fn metadata(&self) -> &Value {
        &self.metadata
    }
}

impl Default for ToolExecutionScope {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolCallRequest {
    /// Creates a new tool call request, generating a fresh call identifier.
    #[must_use]
    pub fn new(tool_name: impl Into<String>, parameters: Value, clock: &impl Clock) -> Self {
        Self {
            call_id: ToolCallId::new(),
            tool_name: tool_name.into(),
            parameters,
            execution_scope: ToolExecutionScope::default(),
            initiated_at: clock.utc(),
        }
    }

    /// Returns the call identifier.
    #[must_use]
    pub const fn call_id(&self) -> ToolCallId {
        self.call_id
    }

    /// Returns the requested tool name.
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Returns the call parameters.
    #[must_use]
    pub const fn parameters(&self) -> &Value {
        &self.parameters
    }

    /// Returns the workflow correlation scope for the call.
    #[must_use]
    pub const fn execution_scope(&self) -> &ToolExecutionScope {
        &self.execution_scope
    }

    /// Returns the initiation timestamp.
    #[must_use]
    pub const fn initiated_at(&self) -> DateTime<Utc> {
        self.initiated_at
    }

    /// Attaches an explicit execution scope to the request.
    #[must_use]
    pub fn with_execution_scope(mut self, execution_scope: ToolExecutionScope) -> Self {
        self.execution_scope = execution_scope;
        self
    }

    /// Associates a task identifier with the request.
    #[must_use]
    pub fn with_task_id(mut self, task_id: TaskId) -> Self {
        self.execution_scope = self.execution_scope.with_task_id(task_id);
        self
    }

    /// Associates a conversation identifier with the request.
    #[must_use]
    pub fn with_conversation_id(mut self, conversation_id: ConversationId) -> Self {
        self.execution_scope = self.execution_scope.with_conversation_id(conversation_id);
        self
    }
}

/// Outcome of a completed tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCallOutcome {
    /// The tool call completed successfully.
    Success {
        /// Content returned by the tool.
        content: Value,
    },
    /// The tool call failed.
    Failure {
        /// Human-readable error description.
        error: String,
    },
}

impl ToolCallOutcome {
    /// Returns `true` when the outcome is a success.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Returns `true` when the outcome is a failure.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::Failure { .. })
    }
}

/// Timing metadata for a completed tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolCallTiming {
    /// How long the call took.
    pub duration: Duration,
    /// When the call completed.
    pub completed_at: DateTime<Utc>,
}

/// Completed tool call result with timing and routing metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallResult {
    call_id: ToolCallId,
    tool_name: String,
    server_id: McpServerId,
    outcome: ToolCallOutcome,
    duration: Duration,
    completed_at: DateTime<Utc>,
}

impl ToolCallResult {
    /// Creates a tool call result from a request, server, outcome, and
    /// timing metadata.
    #[must_use]
    pub fn from_request(
        request: &ToolCallRequest,
        server_id: McpServerId,
        outcome: ToolCallOutcome,
        timing: ToolCallTiming,
    ) -> Self {
        Self {
            call_id: request.call_id(),
            tool_name: request.tool_name().to_owned(),
            server_id,
            outcome,
            duration: timing.duration,
            completed_at: timing.completed_at,
        }
    }

    /// Returns the call identifier.
    #[must_use]
    pub const fn call_id(&self) -> ToolCallId {
        self.call_id
    }

    /// Returns the tool name.
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Returns the server that handled the call.
    #[must_use]
    pub const fn server_id(&self) -> McpServerId {
        self.server_id
    }

    /// Returns the call outcome.
    #[must_use]
    pub const fn outcome(&self) -> &ToolCallOutcome {
        &self.outcome
    }

    /// Returns the call duration.
    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    /// Returns the completion timestamp.
    #[must_use]
    pub const fn completed_at(&self) -> DateTime<Utc> {
        self.completed_at
    }
}

impl From<crate::hook_engine::domain::HookExecutionScope> for ToolExecutionScope {
    fn from(src: crate::hook_engine::domain::HookExecutionScope) -> Self {
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
