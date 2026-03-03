//! Error types for hook engine domain validation.

use thiserror::Error;

/// Errors returned while constructing hook domain values.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum HookDomainError {
    /// The hook identifier is empty after trimming.
    #[error("hook id must not be empty")]
    EmptyHookId,

    /// The hook action identifier is empty after trimming.
    #[error("hook action id must not be empty")]
    EmptyHookActionId,

    /// The hook name is empty after trimming.
    #[error("hook name must not be empty")]
    EmptyHookName,

    /// The hook definition has no actions.
    #[error("hook definition must include at least one action")]
    MissingActions,
}
