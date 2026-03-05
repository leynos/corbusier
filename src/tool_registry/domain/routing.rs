//! Tool call routing domain types.
//!
//! These types model the lifecycle of a routed tool call: the inbound
//! request, the outcome, and the completed result with timing metadata.

use super::McpServerId;
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
    initiated_at: DateTime<Utc>,
}

impl ToolCallRequest {
    /// Creates a new tool call request, generating a fresh call identifier.
    #[must_use]
    pub fn new(tool_name: impl Into<String>, parameters: Value, clock: &impl Clock) -> Self {
        Self {
            call_id: ToolCallId::new(),
            tool_name: tool_name.into(),
            parameters,
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

    /// Returns the initiation timestamp.
    #[must_use]
    pub const fn initiated_at(&self) -> DateTime<Utc> {
        self.initiated_at
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
    /// Creates a new tool call result.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "result struct captures all routing metadata fields"
    )]
    pub fn new(
        call_id: ToolCallId,
        tool_name: impl Into<String>,
        server_id: McpServerId,
        outcome: ToolCallOutcome,
        duration: Duration,
        completed_at: DateTime<Utc>,
    ) -> Self {
        Self {
            call_id,
            tool_name: tool_name.into(),
            server_id,
            outcome,
            duration,
            completed_at,
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
