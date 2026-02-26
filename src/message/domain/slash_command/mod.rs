//! Slash command domain model.

mod definition;
mod error;
mod execution;
mod parser;

pub use definition::{
    CommandParameterSpec, CommandParameterType, SlashCommandDefinition, ToolCallTemplate,
};
pub use error::SlashCommandError;
pub use execution::{PlannedToolCall, SlashCommandExecution};
pub use parser::SlashCommandInvocation;
