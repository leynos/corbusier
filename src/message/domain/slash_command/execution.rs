//! Slash-command execution output types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::parser::SlashCommandInvocation;
use crate::message::domain::{SlashCommandExpansion, ToolCallAudit};

/// A deterministic planned tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedToolCall {
    /// Stable call identifier.
    call_id: String,
    /// Tool name.
    tool_name: String,
    /// Tool arguments payload.
    arguments: Value,
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

    /// Returns the stable call identifier.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// Returns the target tool name.
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Returns rendered JSON arguments.
    #[must_use]
    pub const fn arguments(&self) -> &Value {
        &self.arguments
    }
}

/// Output produced by slash-command execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlashCommandExecution {
    /// Parsed invocation.
    invocation: SlashCommandInvocation,
    /// Expansion metadata for message persistence.
    expansion: SlashCommandExpansion,
    /// Planned deterministic tool calls.
    planned_tool_calls: Vec<PlannedToolCall>,
    /// Auditable tool call records.
    tool_call_audits: Vec<ToolCallAudit>,
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

    /// Returns the parsed slash-command invocation.
    #[must_use]
    pub const fn invocation(&self) -> &SlashCommandInvocation {
        &self.invocation
    }

    /// Returns expansion metadata.
    #[must_use]
    pub const fn expansion(&self) -> &SlashCommandExpansion {
        &self.expansion
    }

    /// Returns deterministic planned tool calls.
    #[must_use]
    pub fn planned_tool_calls(&self) -> &[PlannedToolCall] {
        &self.planned_tool_calls
    }

    /// Returns auditable tool call records.
    #[must_use]
    pub fn tool_call_audits(&self) -> &[ToolCallAudit] {
        &self.tool_call_audits
    }

    /// Consumes the execution and returns expansion metadata plus audits.
    #[must_use]
    pub fn into_expansion_and_audits(self) -> (SlashCommandExpansion, Vec<ToolCallAudit>) {
        (self.expansion, self.tool_call_audits)
    }
}
