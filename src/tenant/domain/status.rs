//! Tenant lifecycle status.

use super::ParseTenantStatusError;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Lifecycle status of a tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TenantStatus {
    /// The tenant is active and operational.
    Active,
    /// The tenant has been suspended and cannot perform operations.
    Suspended,
}

impl TenantStatus {
    /// Returns the canonical storage representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
        }
    }
}

impl fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for TenantStatus {
    type Error = ParseTenantStatusError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            _ => Err(ParseTenantStatusError(value.to_owned())),
        }
    }
}
