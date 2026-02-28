//! Application services for the message subsystem.
//!
//! Services orchestrate domain operations and coordinate between ports,
//! implementing business workflows that span multiple aggregates.

mod handoff;
mod slash_command;

#[cfg(test)]
mod handoff_tests;

pub use handoff::{HandoffService, ServiceInitiateParams};
pub use slash_command::SlashCommandService;
