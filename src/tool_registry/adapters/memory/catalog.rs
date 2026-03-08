//! In-memory repository for tool catalog entries and audit records.

use crate::tool_registry::{
    domain::{CatalogEntry, McpServerId, ToolCallAuditRecord},
    ports::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult},
};
use async_trait::async_trait;
use mockable::DefaultClock;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Thread-safe in-memory tool catalog repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryToolCatalog {
    state: Arc<RwLock<InMemoryToolCatalogState>>,
}

#[derive(Debug, Default)]
struct InMemoryToolCatalogState {
    entries: HashMap<String, CatalogEntry>,
    audit_records: Vec<ToolCallAuditRecord>,
}

impl InMemoryToolCatalog {
    /// Creates an empty in-memory tool catalog.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn apply_to_server_entries(
        &self,
        server_id: McpServerId,
        apply: fn(&mut CatalogEntry, &DefaultClock),
    ) -> ToolCatalogResult<()> {
        let mut state = self
            .state
            .write()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;
        let clock = DefaultClock;
        for entry in state.entries.values_mut() {
            if entry.server_id() == server_id {
                apply(entry, &clock);
            }
        }
        Ok(())
    }

    /// Returns a snapshot of audit records for test assertions.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError::Persistence`] if the lock is poisoned.
    pub fn audit_records(&self) -> ToolCatalogResult<Vec<ToolCallAuditRecord>> {
        let state = self
            .state
            .read()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;
        Ok(state.audit_records.clone())
    }
}

#[async_trait]
impl ToolCatalogRepository for InMemoryToolCatalog {
    async fn sync_server_tools(
        &self,
        server_id: McpServerId,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<()> {
        let mut state = self
            .state
            .write()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;

        // Remove existing entries for this server.
        state
            .entries
            .retain(|_, entry| entry.server_id() != server_id);

        // Check for duplicate tool names from other servers.
        for entry in entries {
            let tool_name = entry.tool().name().to_owned();
            if let Some(existing) = state.entries.get(&tool_name)
                && existing.server_id() != server_id
            {
                return Err(ToolCatalogError::DuplicateEntry(entry.id()));
            }
            state.entries.insert(tool_name, entry.clone());
        }

        Ok(())
    }

    async fn mark_server_tools_unavailable(&self, server_id: McpServerId) -> ToolCatalogResult<()> {
        self.apply_to_server_entries(server_id, CatalogEntry::mark_unavailable)
    }

    async fn mark_server_tools_available(&self, server_id: McpServerId) -> ToolCatalogResult<()> {
        self.apply_to_server_entries(server_id, CatalogEntry::mark_available)
    }

    async fn find_by_tool_name(&self, tool_name: &str) -> ToolCatalogResult<Option<CatalogEntry>> {
        let state = self
            .state
            .read()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;
        Ok(state.entries.get(tool_name).cloned())
    }

    async fn list_all(&self) -> ToolCatalogResult<Vec<CatalogEntry>> {
        let state = self
            .state
            .read()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;
        Ok(state.entries.values().cloned().collect())
    }

    async fn record_audit(&self, record: &ToolCallAuditRecord) -> ToolCatalogResult<()> {
        let mut state = self
            .state
            .write()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;
        state.audit_records.push(record.clone());
        Ok(())
    }
}
