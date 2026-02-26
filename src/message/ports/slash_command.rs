//! Slash-command registry port.
//!
//! The registry port provides command definitions to orchestration services.

use thiserror::Error;

use crate::message::domain::SlashCommandDefinition;

/// Result type for slash-command registry operations.
pub type SlashCommandRegistryResult<T> = Result<T, SlashCommandRegistryError>;

/// Port for loading slash-command definitions.
pub trait SlashCommandRegistry: Send + Sync {
    /// Finds a command definition by command name (without leading slash).
    ///
    /// # Errors
    ///
    /// Returns [`SlashCommandRegistryError`] when registry access fails.
    fn find_by_name(
        &self,
        command: &str,
    ) -> SlashCommandRegistryResult<Option<SlashCommandDefinition>>;

    /// Lists all available command definitions.
    ///
    /// # Errors
    ///
    /// Returns [`SlashCommandRegistryError`] when registry access fails.
    fn list(&self) -> SlashCommandRegistryResult<Vec<SlashCommandDefinition>>;
}

/// Errors for slash-command registry operations.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SlashCommandRegistryError {
    /// The registry contains invalid command definitions.
    #[error("invalid slash-command definition: {0}")]
    InvalidDefinition(String),

    /// General storage or adapter failure.
    #[error("slash-command registry unavailable: {0}")]
    Unavailable(String),
}
