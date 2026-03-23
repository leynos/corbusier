//! Turn-execution domain types for agent backend orchestration.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Domain errors for turn-execution value validation.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TurnDomainError {
    /// Tool names must not be empty.
    #[error("tool name must not be empty")]
    EmptyToolName,
}

/// Canonical tool-call request emitted during a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallRequest {
    tool_name: String,
    parameters: Value,
}

impl ToolCallRequest {
    /// Creates a tool-call request.
    ///
    /// # Errors
    ///
    /// Returns [`TurnDomainError::EmptyToolName`] when `tool_name` is empty
    /// after trimming.
    pub fn new(tool_name: impl Into<String>, parameters: Value) -> Result<Self, TurnDomainError> {
        let normalized_tool_name = tool_name.into().trim().to_owned();
        if normalized_tool_name.is_empty() {
            return Err(TurnDomainError::EmptyToolName);
        }
        Ok(Self {
            tool_name: normalized_tool_name,
            parameters,
        })
    }

    /// Returns the tool name.
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Returns the tool-call parameters.
    #[must_use]
    pub const fn parameters(&self) -> &Value {
        &self.parameters
    }
}

/// Result of routing a single tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallResult {
    call_id: String,
    tool_name: String,
    output: Value,
}

impl ToolCallResult {
    /// Creates a new tool-call result.
    #[must_use]
    pub fn new(call_id: impl Into<String>, tool_name: impl Into<String>, output: Value) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            output,
        }
    }

    /// Returns the call identifier.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// Returns the tool name.
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Returns the routed output.
    #[must_use]
    pub const fn output(&self) -> &Value {
        &self.output
    }
}

/// Tool-call execution status used in audits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallAuditStatus {
    /// Tool call routed and completed successfully.
    Succeeded,
    /// Tool call failed during routing/execution.
    Failed,
}

/// Audit record for a tool call routed by the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallAudit {
    call_id: String,
    tool_name: String,
    status: ToolCallAuditStatus,
    error: Option<String>,
}

impl ToolCallAudit {
    /// Creates an audit entry with required fields.
    #[must_use]
    pub fn new(
        call_id: impl Into<String>,
        tool_name: impl Into<String>,
        status: ToolCallAuditStatus,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            status,
            error: None,
        }
    }

    /// Attaches an error message to a failed audit entry.
    #[must_use]
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.status = ToolCallAuditStatus::Failed;
        self.error = Some(error.into());
        self
    }

    /// Returns the call identifier.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// Returns the tool name.
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Returns the execution status.
    #[must_use]
    pub const fn status(&self) -> ToolCallAuditStatus {
        self.status
    }

    /// Returns the optional error message.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// Canonical turn request for backend orchestration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnExecutionRequest {
    conversation_id: Uuid,
    prompt: String,
    tool_calls: Vec<ToolCallRequest>,
}

impl TurnExecutionRequest {
    /// Creates a turn request.
    #[must_use]
    pub fn new(
        conversation_id: Uuid,
        prompt: impl Into<String>,
        tool_calls: Vec<ToolCallRequest>,
    ) -> Self {
        Self {
            conversation_id,
            prompt: prompt.into(),
            tool_calls,
        }
    }

    /// Returns the conversation identifier.
    #[must_use]
    pub const fn conversation_id(&self) -> Uuid {
        self.conversation_id
    }

    /// Returns the user prompt.
    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }

    /// Returns requested tool calls.
    #[must_use]
    pub fn tool_calls(&self) -> &[ToolCallRequest] {
        &self.tool_calls
    }
}

/// Canonical result returned by an agent runtime for a turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnExecutionResult {
    assistant_response: String,
    tool_calls: Vec<ToolCallRequest>,
}

impl TurnExecutionResult {
    /// Creates a turn result.
    #[must_use]
    pub fn new(assistant_response: impl Into<String>, tool_calls: Vec<ToolCallRequest>) -> Self {
        Self {
            assistant_response: assistant_response.into(),
            tool_calls,
        }
    }

    /// Returns assistant response text.
    #[must_use]
    pub fn assistant_response(&self) -> &str {
        &self.assistant_response
    }

    /// Returns emitted tool calls.
    #[must_use]
    pub fn tool_calls(&self) -> &[ToolCallRequest] {
        &self.tool_calls
    }
}

/// Computes a deterministic call identifier for `tool_call` at `index`.
#[must_use]
pub fn deterministic_tool_call_id(tool_call: &ToolCallRequest, index: usize) -> String {
    let payload = canonical_json_value(&Value::Object(Map::from_iter([
        ("index".to_owned(), Value::from(index)),
        (
            "tool_name".to_owned(),
            Value::from(tool_call.tool_name().to_owned()),
        ),
        (
            "parameters".to_owned(),
            canonical_json_value(tool_call.parameters()),
        ),
    ])))
    .to_string();
    let mut hasher = Sha256::new();
    hasher.update(payload.as_bytes());
    format!("call-{:x}", hasher.finalize())
}

fn canonical_json_value(value: &Value) -> Value {
    as_json_object(value).map_or_else(
        || as_json_array(value).map_or_else(|| value.clone(), canonical_array_value),
        canonical_object_value,
    )
}

fn canonical_object_value(map: &Map<String, Value>) -> Value {
    let mut keys = map.keys().collect::<Vec<_>>();
    keys.sort_unstable();
    let canonical_map = keys
        .into_iter()
        .map(|key| (key.to_owned(), canonical_json_value(&map[key])))
        .collect::<Map<String, Value>>();
    Value::Object(canonical_map)
}

fn canonical_array_value(values: &[Value]) -> Value {
    Value::Array(values.iter().map(canonical_json_value).collect())
}

fn as_json_object(value: &Value) -> Option<&Map<String, Value>> {
    value.as_object()
}

fn as_json_array(value: &Value) -> Option<&[Value]> {
    value.as_array().map(Vec::as_slice)
}
