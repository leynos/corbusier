//! Tool catalog entry domain types.
//!
//! A [`CatalogEntry`] links a discovered [`McpToolDefinition`] to the
//! MCP server that hosts it, tracking availability across server
//! lifecycle transitions.

use super::{McpServerId, McpServerName, McpToolDefinition};
use chrono::{DateTime, Utc};
use mockable::Clock;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a catalog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CatalogEntryId(Uuid);

impl CatalogEntryId {
    /// Creates a new random catalog entry identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a catalog entry identifier from an existing UUID.
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

impl Default for CatalogEntryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CatalogEntryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// A tool discovered from an MCP server and persisted in the catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    id: CatalogEntryId,
    server_id: McpServerId,
    server_name: McpServerName,
    tool: McpToolDefinition,
    available: bool,
    discovered_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CatalogEntry {
    /// Creates a new catalog entry for a discovered tool.
    ///
    /// The entry is initially marked as available.
    #[must_use]
    pub fn new(
        server_id: McpServerId,
        server_name: McpServerName,
        tool: McpToolDefinition,
        clock: &impl Clock,
    ) -> Self {
        let now = clock.utc();
        Self {
            id: CatalogEntryId::new(),
            server_id,
            server_name,
            tool,
            available: true,
            discovered_at: now,
            updated_at: now,
        }
    }

    /// Returns the catalog entry identifier.
    #[must_use]
    pub const fn id(&self) -> CatalogEntryId {
        self.id
    }

    /// Returns the hosting server identifier.
    #[must_use]
    pub const fn server_id(&self) -> McpServerId {
        self.server_id
    }

    /// Returns the hosting server name.
    #[must_use]
    pub const fn server_name(&self) -> &McpServerName {
        &self.server_name
    }

    /// Returns the tool definition.
    #[must_use]
    pub const fn tool(&self) -> &McpToolDefinition {
        &self.tool
    }

    /// Returns whether the tool is currently available for invocation.
    #[must_use]
    pub const fn available(&self) -> bool {
        self.available
    }

    /// Returns the timestamp when the tool was first discovered.
    #[must_use]
    pub const fn discovered_at(&self) -> DateTime<Utc> {
        self.discovered_at
    }

    /// Returns the timestamp of the latest update.
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Marks the tool as available.
    pub fn mark_available(&mut self, clock: &impl Clock) {
        self.available = true;
        self.updated_at = clock.utc();
    }

    /// Marks the tool as unavailable.
    pub fn mark_unavailable(&mut self, clock: &impl Clock) {
        self.available = false;
        self.updated_at = clock.utc();
    }

    /// Reconstructs a catalog entry from persisted storage.
    ///
    /// Unlike [`new`](Self::new), this constructor does not generate a fresh
    /// identifier or timestamp -- all fields are supplied by the persistence
    /// layer.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "reconstruction from persistence requires all stored fields"
    )]
    pub const fn from_persisted(
        id: CatalogEntryId,
        server_id: McpServerId,
        server_name: McpServerName,
        tool: McpToolDefinition,
        available: bool,
        discovered_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            server_id,
            server_name,
            tool,
            available,
            discovered_at,
            updated_at,
        }
    }

    /// Replaces the tool definition with an updated version.
    pub fn update_tool(&mut self, tool: McpToolDefinition, clock: &impl Clock) {
        self.tool = tool;
        self.updated_at = clock.utc();
    }
}
