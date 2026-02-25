//! Agent backend provider information.

use super::BackendDomainError;
use serde::{Deserialize, Serialize};

/// Descriptive metadata about an agent backend provider.
///
/// Stored as JSONB alongside capabilities so new fields can be added
/// without database migrations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendInfo {
    display_name: String,
    version: String,
    provider: String,
}

impl BackendInfo {
    /// Creates validated backend info.
    ///
    /// All three fields are trimmed. Empty values after trimming are rejected.
    ///
    /// # Errors
    ///
    /// Returns [`BackendDomainError::EmptyDisplayName`],
    /// [`BackendDomainError::EmptyVersion`], or
    /// [`BackendDomainError::EmptyProvider`] when the corresponding field is
    /// blank.
    pub fn new(
        raw_display_name: impl Into<String>,
        raw_version: impl Into<String>,
        raw_provider: impl Into<String>,
    ) -> Result<Self, BackendDomainError> {
        let display_name = raw_display_name.into().trim().to_owned();
        let version = raw_version.into().trim().to_owned();
        let provider = raw_provider.into().trim().to_owned();

        if display_name.is_empty() {
            return Err(BackendDomainError::EmptyDisplayName);
        }
        if version.is_empty() {
            return Err(BackendDomainError::EmptyVersion);
        }
        if provider.is_empty() {
            return Err(BackendDomainError::EmptyProvider);
        }

        Ok(Self {
            display_name,
            version,
            provider,
        })
    }

    /// Returns the human-readable backend name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the backend version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the backend provider name.
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.provider
    }
}
