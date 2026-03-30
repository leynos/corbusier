//! Application services for the message subsystem.
//!
//! Services orchestrate domain operations and coordinate between ports,
//! implementing business workflows that span multiple aggregates.

mod conversation;
mod handoff;
mod slash_command;

#[cfg(test)]
mod conversation_tests;
#[cfg(test)]
mod handoff_tests;

pub use conversation::{AppendMessageRequest, ConversationService, ConversationServiceError};
pub use handoff::{CompleteHandoffParams, HandoffService, ServiceInitiateParams};
pub use slash_command::SlashCommandService;
