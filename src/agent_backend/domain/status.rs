//! Backend lifecycle status.

use super::ParseBackendStatusError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Lifecycle status of a registered agent backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendStatus {
    /// The backend is available for use.
    Active,
    /// The backend has been deactivated and is excluded from active listings.
    Inactive,
}

impl BackendStatus {
    /// Returns the canonical storage representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }
}

impl fmt::Display for BackendStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for BackendStatus {
    type Error = ParseBackendStatusError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            _ => Err(ParseBackendStatusError(value.to_owned())),
        }
    }
}
