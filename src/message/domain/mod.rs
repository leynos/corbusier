//! Domain types for the message subsystem.
//!
//! This module contains pure domain types with no infrastructure dependencies.
//! All types are immutable after construction and serialisable via serde.

mod agent_session;
mod audit;
mod content;
mod context_snapshot;
mod handoff;
mod ids;
mod message;
mod metadata;
mod role;
mod slash_command;

#[cfg(test)]
mod agent_session_tests;
#[cfg(test)]
mod handoff_tests;

pub use agent_session::{
    AgentSession, AgentSessionState, HandoffSessionParams, ParseAgentSessionStateError,
};
pub use audit::{AgentResponseAudit, AgentResponseStatus, ToolCallAudit, ToolCallStatus};
pub use content::{AttachmentPart, ContentPart, TextPart, ToolCallPart, ToolResultPart};
pub use context_snapshot::{
    ContextWindowSnapshot, MessageSummary, ParseSnapshotTypeError, SequenceRange, SnapshotParams,
    SnapshotType,
};
pub use handoff::{
    HandoffMetadata, HandoffParams, HandoffStatus, ParseHandoffStatusError, ToolCallReference,
};
pub use ids::{AgentSessionId, ConversationId, HandoffId, MessageId, SequenceNumber, TurnId};
pub use message::{Message, MessageBuilder, MessageBuilderError};
pub use metadata::{MessageMetadata, SlashCommandExpansion};
pub use role::{ParseRoleError, Role};
pub use slash_command::{
    CommandParameterSpec, CommandParameterType, PlannedToolCall, SlashCommandDefinition,
    SlashCommandError, SlashCommandExecution, SlashCommandInvocation,
    SlashCommandRegistryUnavailableError, SlashCommandSchemaError, ToolCallTemplate,
};
