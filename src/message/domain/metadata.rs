//! Message metadata types capturing contextual information about messages.

use super::handoff::HandoffMetadata;
use super::{AgentSessionId, TurnId, audit::AgentResponseAudit, audit::ToolCallAudit};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Metadata associated with a message.
///
/// Captures information about the message's origin, processing context,
/// and any extension data required by specific workflows.
///
/// # Examples
///
/// ```
/// use corbusier::message::domain::{MessageMetadata, TurnId};
///
/// let metadata = MessageMetadata::with_agent_backend("claude_code_sdk")
///     .with_turn_id(TurnId::new());
/// assert_eq!(metadata.agent_backend, Some("claude_code_sdk".to_string()));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// The agent backend that produced this message (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_backend: Option<String>,

    /// The turn identifier within which this message was created.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,

    /// Slash command expansion details (if this message resulted from a command).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slash_command_expansion: Option<SlashCommandExpansion>,

    /// Audit metadata for tool calls associated with this message.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_call_audits: Vec<ToolCallAudit>,

    /// Audit metadata for the agent response associated with this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_response_audit: Option<AgentResponseAudit>,

    /// Handoff metadata if this message is part of a handoff.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_metadata: Option<HandoffMetadata>,

    /// The agent session ID for this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_session_id: Option<AgentSessionId>,

    /// Extension data for custom metadata fields.
    ///
    /// Extensions are serialized under an explicit `"extensions"` key rather
    /// than being flattened into the top-level JSON object, preventing key
    /// collisions with known struct fields.  Workflow-specific data should
    /// use a reserved, versioned namespace key such as `"review.linkage.v1"`
    /// so that schema evolution and deserialization remain predictable.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, Value>,
}

impl MessageMetadata {
    /// Creates empty metadata.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates metadata with an agent backend specified.
    #[must_use]
    pub fn with_agent_backend(agent_backend: impl Into<String>) -> Self {
        Self {
            agent_backend: Some(agent_backend.into()),
            ..Default::default()
        }
    }

    /// Sets the turn identifier.
    #[must_use]
    pub const fn with_turn_id(mut self, turn_id: TurnId) -> Self {
        self.turn_id = Some(turn_id);
        self
    }

    /// Sets the slash command expansion details.
    #[must_use]
    pub fn with_slash_command_expansion(mut self, expansion: SlashCommandExpansion) -> Self {
        self.slash_command_expansion = Some(expansion);
        self
    }

    /// Appends a tool call audit record.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{MessageMetadata, ToolCallAudit, ToolCallStatus};
    ///
    /// let metadata = MessageMetadata::empty()
    ///     .with_tool_call_audit(ToolCallAudit::new(
    ///         "call-123",
    ///         "read_file",
    ///         ToolCallStatus::Succeeded,
    ///     ));
    /// assert_eq!(metadata.tool_call_audits.len(), 1);
    /// ```
    #[must_use]
    pub fn with_tool_call_audit(mut self, audit: ToolCallAudit) -> Self {
        self.tool_call_audits.push(audit);
        self
    }

    /// Appends multiple tool call audit records.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{MessageMetadata, ToolCallAudit, ToolCallStatus};
    ///
    /// let audits = vec![
    ///     ToolCallAudit::new("call-1", "search", ToolCallStatus::Queued),
    ///     ToolCallAudit::new("call-2", "read_file", ToolCallStatus::Running),
    /// ];
    /// let metadata = MessageMetadata::empty().with_tool_call_audits(audits);
    /// assert_eq!(metadata.tool_call_audits.len(), 2);
    /// ```
    #[must_use]
    pub fn with_tool_call_audits(
        mut self,
        audits: impl IntoIterator<Item = ToolCallAudit>,
    ) -> Self {
        self.tool_call_audits.extend(audits);
        self
    }

    /// Sets the agent response audit metadata.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{AgentResponseAudit, AgentResponseStatus, MessageMetadata};
    ///
    /// let response = AgentResponseAudit::new(AgentResponseStatus::Completed);
    /// let metadata = MessageMetadata::empty().with_agent_response_audit(response);
    /// assert!(metadata.agent_response_audit.is_some());
    /// ```
    #[must_use]
    pub fn with_agent_response_audit(mut self, audit: AgentResponseAudit) -> Self {
        self.agent_response_audit = Some(audit);
        self
    }

    /// Adds an extension field.
    #[must_use]
    pub fn with_extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }

    /// Adds structured review linkage data under the reserved, versioned
    /// namespace key `"review.linkage.v1"`.
    ///
    /// This groups all review-specific anchor fields into a single JSON
    /// object, preventing key collisions with top-level extension keys
    /// and making schema evolution predictable.
    ///
    /// # Errors
    ///
    /// Returns `serde_json::Error` if the `ReviewLinkage` fails to
    /// serialize.  In practice this cannot happen because every field is
    /// a `String` or `Option<String>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use corbusier::message::domain::{MessageMetadata, ReviewLinkage};
    ///
    /// let linkage = ReviewLinkage::new("rc-42", "thread-root-7", "alice", "pending")
    ///     .with_file_path("src/lib.rs")
    ///     .with_commit_sha("abc123");
    /// let metadata = MessageMetadata::empty()
    ///     .with_review_linkage(&linkage)
    ///     .unwrap();
    /// let ext = metadata.extensions.get("review.linkage.v1").unwrap();
    /// assert_eq!(ext["review_comment_id"], "rc-42");
    /// assert_eq!(ext["reviewer"], "alice");
    /// ```
    pub fn with_review_linkage(self, linkage: &ReviewLinkage) -> Result<Self, serde_json::Error> {
        let value = serde_json::to_value(linkage)?;
        Ok(self.with_extension("review.linkage.v1", value))
    }

    /// Sets the handoff metadata.
    #[must_use]
    pub fn with_handoff_metadata(mut self, handoff: HandoffMetadata) -> Self {
        self.handoff_metadata = Some(handoff);
        self
    }

    /// Sets the agent session ID.
    #[must_use]
    pub const fn with_agent_session_id(mut self, session_id: AgentSessionId) -> Self {
        self.agent_session_id = Some(session_id);
        self
    }

    /// Returns `true` if the metadata is empty (no fields set).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.agent_backend.is_none()
            && self.turn_id.is_none()
            && self.slash_command_expansion.is_none()
            && self.tool_call_audits.is_empty()
            && self.agent_response_audit.is_none()
            && self.handoff_metadata.is_none()
            && self.agent_session_id.is_none()
            && self.extensions.is_empty()
    }
}

/// Details about a slash command expansion that produced a message.
///
/// When a user invokes a slash command (e.g., `/review`), the command is
/// expanded into a template that generates one or more messages. This
/// structure records the expansion details for audit and debugging.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlashCommandExpansion {
    /// The original command string (e.g., "/review").
    pub command: String,
    /// Parameters passed to the command.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub parameters: HashMap<String, Value>,
    /// The expanded template result.
    pub expanded_content: String,
}

impl SlashCommandExpansion {
    /// Creates a new slash command expansion record.
    #[must_use]
    pub fn new(command: impl Into<String>, expanded_content: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            parameters: HashMap::new(),
            expanded_content: expanded_content.into(),
        }
    }

    /// Adds a parameter to the expansion.
    #[must_use]
    pub fn with_parameter(mut self, key: impl Into<String>, value: Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }
}

/// Structured review linkage data stored under the reserved, versioned
/// namespace key `"review.linkage.v1"` inside `MessageMetadata.extensions`.
///
/// Groups review-comment anchoring fields into a single typed object so
/// that schema evolution and deserialization remain predictable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewLinkage {
    /// Identifier of the review comment in the external VCS provider.
    pub review_comment_id: String,
    /// Root comment identifier that anchors the review thread.
    pub thread_root_id: String,
    /// Login or display name of the reviewer.
    pub reviewer: String,
    /// Source file path the comment is anchored to (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Commit SHA the comment is anchored to (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    /// Current verification status of the review linkage.
    pub verification_status: String,
}

impl ReviewLinkage {
    /// Creates a new review linkage with the required fields.
    #[must_use]
    pub fn new(
        review_comment_id: impl Into<String>,
        thread_root_id: impl Into<String>,
        reviewer: impl Into<String>,
        verification_status: impl Into<String>,
    ) -> Self {
        Self {
            review_comment_id: review_comment_id.into(),
            thread_root_id: thread_root_id.into(),
            reviewer: reviewer.into(),
            file_path: None,
            commit_sha: None,
            verification_status: verification_status.into(),
        }
    }

    /// Sets the file path anchor.
    #[must_use]
    pub fn with_file_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Sets the commit SHA anchor.
    #[must_use]
    pub fn with_commit_sha(mut self, sha: impl Into<String>) -> Self {
        self.commit_sha = Some(sha.into());
        self
    }
}
