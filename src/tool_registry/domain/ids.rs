//! Identifier and validated-name types for MCP servers.

use super::ToolRegistryDomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Maximum length for an MCP server name, matching `VARCHAR(100)`.
const MAX_SERVER_NAME_LENGTH: usize = 100;

/// Unique identifier for an MCP server registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct McpServerId(Uuid);

impl McpServerId {
    /// Creates a new random MCP server identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an MCP server identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the wrapped UUID.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for McpServerId {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<Uuid> for McpServerId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for McpServerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Validated MCP server name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct McpServerName(String);

impl McpServerName {
    /// Creates a validated MCP server name.
    ///
    /// The input is trimmed and lowercased. Only characters in `[a-z0-9_]`
    /// are accepted.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRegistryDomainError`] when validation fails.
    pub fn new(value: impl Into<String>) -> Result<Self, ToolRegistryDomainError> {
        let normalized = value.into().trim().to_ascii_lowercase();

        if normalized.is_empty() {
            return Err(ToolRegistryDomainError::EmptyServerName);
        }

        let is_valid = normalized.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        });
        if !is_valid {
            return Err(ToolRegistryDomainError::InvalidServerName(normalized));
        }

        if normalized.len() > MAX_SERVER_NAME_LENGTH {
            return Err(ToolRegistryDomainError::ServerNameTooLong(normalized));
        }

        Ok(Self(normalized))
    }

    /// Returns the MCP server name as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for McpServerName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for McpServerName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
