//! Error types for agent backend domain validation and parsing.

use thiserror::Error;

/// Errors returned while constructing agent backend domain values.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum BackendDomainError {
    /// The backend name is empty after trimming.
    #[error("backend name must not be empty")]
    EmptyBackendName,

    /// The backend name contains characters outside `[a-z0-9_]`.
    #[error(
        "backend name '{0}' contains invalid characters (only lowercase alphanumeric and underscores allowed)"
    )]
    InvalidBackendName(String),

    /// The backend name exceeds the 100-character storage limit.
    #[error("backend name exceeds 100 character limit: {0}")]
    BackendNameTooLong(String),

    /// The backend info display name is empty after trimming.
    #[error("backend info display name must not be empty")]
    EmptyDisplayName,

    /// The backend info version is empty after trimming.
    #[error("backend info version must not be empty")]
    EmptyVersion,

    /// The backend info provider is empty after trimming.
    #[error("backend info provider must not be empty")]
    EmptyProvider,
}

/// Error returned while parsing backend status from persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown backend status: {0}")]
pub struct ParseBackendStatusError(pub String);
