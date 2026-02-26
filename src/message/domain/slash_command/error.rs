//! Error types for slash-command parsing and execution.

use thiserror::Error;

/// Errors for slash-command parsing, validation, and execution.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SlashCommandError {
    /// Input was empty.
    #[error("slash command input cannot be empty")]
    EmptyInput,

    /// Input does not start with `/`.
    #[error("slash commands must start with '/'")]
    MissingLeadingSlash,

    /// Command name is invalid.
    #[error("invalid command name '{0}'")]
    InvalidCommandName(String),

    /// A parameter token does not match `key=value`.
    #[error("invalid parameter token '{token}': expected key=value")]
    InvalidParameterToken {
        /// The malformed token text.
        token: String,
    },

    /// A quoted string was not terminated.
    #[error("unterminated quoted value in slash command")]
    UnterminatedQuotedValue,

    /// Duplicate parameter key.
    #[error("duplicate parameter '{0}'")]
    DuplicateParameter(String),

    /// Command is not known by the registry.
    #[error("command '/{0}' was not found")]
    UnknownCommand(String),

    /// Parameter does not exist on command definition.
    #[error("unknown parameter '{parameter}' for command '/{command}'")]
    UnknownParameter {
        /// Command name.
        command: String,
        /// Unknown parameter name.
        parameter: String,
    },

    /// Required parameter missing.
    #[error("missing required parameter '{parameter}' for command '/{command}'")]
    MissingRequiredParameter {
        /// Command name.
        command: String,
        /// Missing parameter name.
        parameter: String,
    },

    /// Parameter value is invalid.
    #[error("invalid value for parameter '{parameter}' in command '/{command}': {reason}")]
    InvalidParameterValue {
        /// Command name.
        command: String,
        /// Parameter name.
        parameter: String,
        /// Validation reason.
        reason: String,
    },

    /// Parameter schema is invalid.
    #[error("invalid parameter definition for '{parameter}' in command '/{command}': {reason}")]
    InvalidParameterDefinition {
        /// Command name.
        command: String,
        /// Parameter name.
        parameter: String,
        /// Validation reason.
        reason: String,
    },

    /// Template rendering failed.
    #[error("template rendering failed for command '/{command}': {reason}")]
    TemplateRender {
        /// Command name.
        command: String,
        /// Rendering failure reason.
        reason: String,
    },

    /// Rendered tool arguments were not valid JSON.
    #[error("template output for tool '{tool_name}' must be valid JSON: {reason}")]
    InvalidToolArgumentsTemplate {
        /// Tool name.
        tool_name: String,
        /// Parse failure reason.
        reason: String,
    },

    /// Registry operation failed.
    #[error("slash-command registry error: {0}")]
    Registry(String),
}
