//! MCP server health status domain types.

use super::ParseMcpServerHealthStatusError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Health status of an MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerHealthStatus {
    /// Health has not been checked yet.
    Unknown,
    /// Server is reachable and healthy.
    Healthy,
    /// Server is reachable but unhealthy.
    Unhealthy,
}

impl McpServerHealthStatus {
    /// Returns the canonical storage representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Healthy => "healthy",
            Self::Unhealthy => "unhealthy",
        }
    }
}

impl fmt::Display for McpServerHealthStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<&str> for McpServerHealthStatus {
    type Error = ParseMcpServerHealthStatusError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "unknown" => Ok(Self::Unknown),
            "healthy" => Ok(Self::Healthy),
            "unhealthy" => Ok(Self::Unhealthy),
            _ => Err(ParseMcpServerHealthStatusError(value.to_owned())),
        }
    }
}

/// Timestamped health snapshot for an MCP server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerHealthSnapshot {
    status: McpServerHealthStatus,
    checked_at: DateTime<Utc>,
    message: Option<String>,
}

impl McpServerHealthSnapshot {
    /// Creates a health snapshot.
    #[must_use]
    pub const fn new(status: McpServerHealthStatus, checked_at: DateTime<Utc>) -> Self {
        Self {
            status,
            checked_at,
            message: None,
        }
    }

    /// Creates an `unknown` health snapshot.
    #[must_use]
    pub const fn unknown(checked_at: DateTime<Utc>) -> Self {
        Self::new(McpServerHealthStatus::Unknown, checked_at)
    }

    /// Creates a `healthy` health snapshot.
    #[must_use]
    pub const fn healthy(checked_at: DateTime<Utc>) -> Self {
        Self::new(McpServerHealthStatus::Healthy, checked_at)
    }

    /// Creates an `unhealthy` health snapshot with details.
    #[must_use]
    pub fn unhealthy(checked_at: DateTime<Utc>, message: impl Into<String>) -> Self {
        Self::new(McpServerHealthStatus::Unhealthy, checked_at).with_message(message)
    }

    /// Adds an explanatory message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        let normalized = message.into().trim().to_owned();
        if !normalized.is_empty() {
            self.message = Some(normalized);
        }
        self
    }

    /// Returns the health status.
    #[must_use]
    pub const fn status(&self) -> McpServerHealthStatus {
        self.status
    }

    /// Returns the health check timestamp.
    #[must_use]
    pub const fn checked_at(&self) -> DateTime<Utc> {
        self.checked_at
    }

    /// Returns an optional health detail message.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
