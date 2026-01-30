//! Audit metadata types for tool calls and agent responses.

use serde::{Deserialize, Serialize};

/// Status values for tool call auditing.
///
/// These values track the lifecycle of a tool call within a conversation.
///
/// # Examples
///
/// ```rust
/// use corbusier::message::domain::ToolCallStatus;
///
/// let status = ToolCallStatus::Succeeded;
/// assert_eq!(status, ToolCallStatus::Succeeded);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    /// The tool call is queued but not yet running.
    Queued,
    /// The tool call is currently executing.
    Running,
    /// The tool call completed successfully.
    Succeeded,
    /// The tool call failed.
    Failed,
}

/// Audit metadata for a tool call emitted by an agent.
///
/// This data supplements the canonical tool call content part with
/// status and provenance data required for audit trails.
///
/// # Examples
///
/// ```rust
/// use corbusier::message::domain::{ToolCallAudit, ToolCallStatus};
///
/// let audit = ToolCallAudit::new("call-123", "read_file", ToolCallStatus::Succeeded);
/// assert_eq!(audit.call_id, "call-123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallAudit {
    /// The tool call identifier.
    pub call_id: String,
    /// The tool name that was invoked.
    pub tool_name: String,
    /// The tool call status.
    pub status: ToolCallStatus,
    /// Optional error details when a tool call fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolCallAudit {
    /// Creates a new tool call audit record.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{ToolCallAudit, ToolCallStatus};
    ///
    /// let audit = ToolCallAudit::new("call-123", "search", ToolCallStatus::Running);
    /// assert_eq!(audit.tool_name, "search");
    /// ```
    #[must_use]
    pub fn new(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        status: ToolCallStatus,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            status,
            error: None,
        }
    }

    /// Adds an error message for a failed tool call.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{ToolCallAudit, ToolCallStatus};
    ///
    /// let audit = ToolCallAudit::new("call-123", "read_file", ToolCallStatus::Failed)
    ///     .with_error("permission denied");
    /// assert_eq!(audit.error, Some("permission denied".to_owned()));
    /// ```
    #[must_use]
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

/// Status values for agent response auditing.
///
/// # Examples
///
/// ```rust
/// use corbusier::message::domain::AgentResponseStatus;
///
/// let status = AgentResponseStatus::Completed;
/// assert_eq!(status, AgentResponseStatus::Completed);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentResponseStatus {
    /// The agent completed the response successfully.
    Completed,
    /// The agent response failed.
    Failed,
    /// The agent response was cancelled or interrupted.
    Cancelled,
}

/// Audit metadata for an agent response.
///
/// # Examples
///
/// ```rust
/// use corbusier::message::domain::{AgentResponseAudit, AgentResponseStatus};
///
/// let audit = AgentResponseAudit::new(AgentResponseStatus::Completed)
///     .with_response_id("resp-456");
/// assert_eq!(audit.response_id.as_deref(), Some("resp-456"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentResponseAudit {
    /// The response status.
    pub status: AgentResponseStatus,
    /// Optional response identifier from the agent backend.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
    /// Optional agent model identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Optional error details when response generation fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AgentResponseAudit {
    /// Creates a new agent response audit record.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{AgentResponseAudit, AgentResponseStatus};
    ///
    /// let audit = AgentResponseAudit::new(AgentResponseStatus::Failed)
    ///     .with_error("timeout");
    /// assert_eq!(audit.error, Some("timeout".to_owned()));
    /// ```
    #[must_use]
    pub const fn new(status: AgentResponseStatus) -> Self {
        Self {
            status,
            response_id: None,
            model: None,
            error: None,
        }
    }

    /// Sets the response identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{AgentResponseAudit, AgentResponseStatus};
    ///
    /// let audit = AgentResponseAudit::new(AgentResponseStatus::Completed)
    ///     .with_response_id("resp-123");
    /// assert_eq!(audit.response_id, Some("resp-123".to_owned()));
    /// ```
    #[must_use]
    pub fn with_response_id(mut self, response_id: impl Into<String>) -> Self {
        self.response_id = Some(response_id.into());
        self
    }

    /// Sets the agent model identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{AgentResponseAudit, AgentResponseStatus};
    ///
    /// let audit = AgentResponseAudit::new(AgentResponseStatus::Completed)
    ///     .with_model("claude-3-opus");
    /// assert_eq!(audit.model, Some("claude-3-opus".to_owned()));
    /// ```
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Adds an error message to the response audit record.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{AgentResponseAudit, AgentResponseStatus};
    ///
    /// let audit = AgentResponseAudit::new(AgentResponseStatus::Failed)
    ///     .with_error("model unavailable");
    /// assert_eq!(audit.error, Some("model unavailable".to_owned()));
    /// ```
    #[must_use]
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}
