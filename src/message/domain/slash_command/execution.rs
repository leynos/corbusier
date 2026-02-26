//! Slash-command execution output types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::parser::SlashCommandInvocation;
use crate::message::domain::{SlashCommandExpansion, ToolCallAudit};

/// A deterministic planned tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedToolCall {
    /// Stable call identifier.
    pub call_id: String,
    /// Tool name.
    pub tool_name: String,
    /// Tool arguments payload.
    pub arguments: Value,
}

impl PlannedToolCall {
    /// Creates a planned tool call.
    #[must_use]
    pub fn new(call_id: impl Into<String>, tool_name: impl Into<String>, arguments: Value) -> Self {
        Self {
            call_id: call_id.into(),
            tool_name: tool_name.into(),
            arguments,
        }
    }
}

/// Output produced by slash-command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlashCommandExecution {
    /// Parsed invocation.
    pub invocation: SlashCommandInvocation,
    /// Expansion metadata for message persistence.
    pub expansion: SlashCommandExpansion,
    /// Planned deterministic tool calls.
    pub planned_tool_calls: Vec<PlannedToolCall>,
    /// Auditable tool call records.
    pub tool_call_audits: Vec<ToolCallAudit>,
}

impl SlashCommandExecution {
    /// Creates a slash-command execution result.
    #[must_use]
    pub const fn new(
        invocation: SlashCommandInvocation,
        expansion: SlashCommandExpansion,
        planned_tool_calls: Vec<PlannedToolCall>,
        tool_call_audits: Vec<ToolCallAudit>,
    ) -> Self {
        Self {
            invocation,
            expansion,
            planned_tool_calls,
            tool_call_audits,
        }
    }
}
