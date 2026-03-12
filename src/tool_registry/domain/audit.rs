//! Tool call audit trail domain types.
//!
//! A [`ToolCallAuditRecord`] captures the full context of a completed
//! tool call invocation for observability and compliance purposes.

use super::McpServerId;
use super::routing::{ToolCallId, ToolCallOutcome, ToolCallResult};
use serde_json::Value;
use std::time::Duration;

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Immutable audit trail entry for a tool call invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallAuditRecord {
    id: Uuid,
    call_id: ToolCallId,
    tool_name: String,
    server_id: McpServerId,
    parameters: Value,
    outcome: ToolCallOutcome,
    duration: Duration,
    initiated_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    stderr_log_path: Option<String>,
}

impl ToolCallAuditRecord {
    /// Builds an audit record from a completed tool call result and the
    /// original request parameters.
    #[must_use]
    pub fn from_result(
        result: &ToolCallResult,
        parameters: Value,
        initiated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            call_id: result.call_id(),
            tool_name: result.tool_name().to_owned(),
            server_id: result.server_id(),
            parameters,
            outcome: result.outcome().clone(),
            duration: result.duration(),
            initiated_at,
            completed_at: result.completed_at(),
            stderr_log_path: None,
        }
    }

    /// Builds an audit record for a pre-execution rejection (e.g.
    /// unavailable tool, schema validation failure, or policy denial).
    #[must_use]
    pub fn for_rejection(
        request: &super::routing::ToolCallRequest,
        server_id: McpServerId,
        error: &dyn std::fmt::Display,
        completed_at: DateTime<Utc>,
    ) -> Self {
        let duration = (completed_at - request.initiated_at())
            .to_std()
            .unwrap_or_default();
        Self {
            id: Uuid::new_v4(),
            call_id: request.call_id(),
            tool_name: request.tool_name().to_owned(),
            server_id,
            parameters: request.parameters().clone(),
            outcome: ToolCallOutcome::Failure {
                error: error.to_string(),
            },
            duration,
            initiated_at: request.initiated_at(),
            completed_at,
            stderr_log_path: None,
        }
    }

    /// Attaches the object store path of captured stderr output.
    #[must_use]
    pub fn with_stderr_log_path(mut self, path: impl Into<String>) -> Self {
        self.stderr_log_path = Some(path.into());
        self
    }

    /// Returns the audit record identifier.
    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    /// Returns the originating call identifier.
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

    /// Returns the call parameters.
    #[must_use]
    pub const fn parameters(&self) -> &Value {
        &self.parameters
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

    /// Returns the initiation timestamp.
    #[must_use]
    pub const fn initiated_at(&self) -> DateTime<Utc> {
        self.initiated_at
    }

    /// Returns the completion timestamp.
    #[must_use]
    pub const fn completed_at(&self) -> DateTime<Utc> {
        self.completed_at
    }

    /// Returns the object store path of captured stderr, if any.
    #[must_use]
    pub fn stderr_log_path(&self) -> Option<&str> {
        self.stderr_log_path.as_deref()
    }
}
