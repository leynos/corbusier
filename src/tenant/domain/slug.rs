//! Validated tenant slug type.

use super::TenantDomainError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum length for a tenant slug, matching DNS label constraints.
const MAX_SLUG_LENGTH: usize = 63;

/// Validated tenant slug suitable for URLs and configuration keys.
///
/// Tenant slugs are lowercased, 1-63 characters, containing only `[a-z0-9-]`.
/// They must start and end with an alphanumeric character and must not contain
/// consecutive hyphens. These rules follow DNS label conventions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantSlug(String);

impl TenantSlug {
    /// Creates a validated tenant slug.
    ///
    /// The input is trimmed and lowercased. Only characters in `[a-z0-9-]` are
    /// accepted. The slug must start and end with an alphanumeric character and
    /// must not contain consecutive hyphens.
    ///
    /// # Errors
    ///
    /// Returns [`TenantDomainError::EmptySlug`] when the value is empty after
    /// trimming, [`TenantDomainError::InvalidSlug`] when it contains characters
    /// outside `[a-z0-9-]`, [`TenantDomainError::SlugBoundaryHyphen`] when it
    /// starts or ends with a hyphen,
    /// [`TenantDomainError::SlugConsecutiveHyphens`] when it contains
    /// consecutive hyphens, or [`TenantDomainError::SlugTooLong`] when the
    /// normalised value exceeds 63 characters.
    pub fn new(value: impl Into<String>) -> Result<Self, TenantDomainError> {
        let raw = value.into();
        let normalized = raw.trim().to_ascii_lowercase();

        if normalized.is_empty() {
            return Err(TenantDomainError::EmptySlug);
        }

        let is_valid = normalized
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');

        if !is_valid {
            return Err(TenantDomainError::InvalidSlug(normalized));
        }

        if normalized.starts_with('-') || normalized.ends_with('-') {
            return Err(TenantDomainError::SlugBoundaryHyphen(normalized));
        }

        if normalized.contains("--") {
            return Err(TenantDomainError::SlugConsecutiveHyphens(normalized));
        }

        if normalized.len() > MAX_SLUG_LENGTH {
            return Err(TenantDomainError::SlugTooLong(normalized));
        }

        Ok(Self(normalized))
    }

    /// Returns the tenant slug as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for TenantSlug {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for TenantSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
