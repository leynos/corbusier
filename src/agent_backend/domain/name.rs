//! Validated backend name type.

use super::BackendDomainError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Maximum length for a backend name, matching the `VARCHAR(100)` column.
const MAX_NAME_LENGTH: usize = 100;

/// Validated, lowercase alphanumeric-plus-underscores backend identifier.
///
/// Backend names are used as unique human-readable identifiers for registered
/// agent backends (e.g. `claude_code_sdk`, `codex_cli`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BackendName(String);

impl BackendName {
    /// Creates a validated backend name.
    ///
    /// The input is trimmed and lowercased. Only characters in `[a-z0-9_]` are
    /// accepted.
    ///
    /// # Errors
    ///
    /// Returns [`BackendDomainError::EmptyBackendName`] when the value is empty
    /// after trimming, [`BackendDomainError::InvalidBackendName`] when it
    /// contains characters outside `[a-z0-9_]`, or
    /// [`BackendDomainError::BackendNameTooLong`] when it exceeds 100
    /// characters.
    pub fn new(value: impl Into<String>) -> Result<Self, BackendDomainError> {
        let raw = value.into();
        let normalized = raw.trim().to_ascii_lowercase();

        if normalized.is_empty() {
            return Err(BackendDomainError::EmptyBackendName);
        }

        if normalized.len() > MAX_NAME_LENGTH {
            return Err(BackendDomainError::BackendNameTooLong(raw));
        }

        let is_valid = normalized
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');

        if !is_valid {
            return Err(BackendDomainError::InvalidBackendName(raw));
        }

        Ok(Self(normalized))
    }

    /// Returns the backend name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for BackendName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for BackendName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
