//! Error types for tenant domain validation and parsing.

use thiserror::Error;

/// Errors returned while constructing tenant domain values.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum TenantDomainError {
    /// The tenant slug is empty after trimming.
    #[error("tenant slug must not be empty")]
    EmptySlug,

    /// The tenant slug contains characters outside `[a-z0-9-]`.
    #[error(
        "tenant slug '{0}' contains invalid characters (only lowercase alphanumeric and hyphens allowed)"
    )]
    InvalidSlug(String),

    /// The tenant slug starts or ends with a hyphen.
    #[error("tenant slug '{0}' must start and end with an alphanumeric character")]
    SlugBoundaryHyphen(String),

    /// The tenant slug contains consecutive hyphens.
    #[error("tenant slug '{0}' must not contain consecutive hyphens")]
    SlugConsecutiveHyphens(String),

    /// The tenant slug exceeds the 63-character storage limit.
    #[error("tenant slug exceeds 63-character limit: {0}")]
    SlugTooLong(String),

    /// The tenant display name is empty after trimming.
    #[error("tenant display name must not be empty")]
    EmptyDisplayName,
}

/// Error returned while parsing tenant status from persistence.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown tenant status: {0}")]
pub struct ParseTenantStatusError(pub String);
