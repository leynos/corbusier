//! In-memory repository for tool catalog entries and audit records.

use crate::context::RequestContext;
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
        _ctx: &RequestContext,
        server_id: McpServerId,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<()> {
        let mut state = self
            .state
            .write()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;

        // Stage the new entries, checking for cross-server duplicates
        // before mutating live state.
        let mut staged: HashMap<String, CatalogEntry> = HashMap::new();
        for entry in entries {
            let tool_name = entry.tool().name().to_owned();
            if let Some(existing) = state.entries.get(&tool_name)
                && existing.server_id() != server_id
            {
                return Err(ToolCatalogError::DuplicateEntry(entry.id()));
            }
            staged.insert(tool_name, entry.clone());
        }

        // Validation passed -- swap atomically.
        state
            .entries
            .retain(|_, entry| entry.server_id() != server_id);
        state.entries.extend(staged);

        Ok(())
    }

    async fn mark_server_tools_unavailable(
        &self,
        _ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()> {
        self.apply_to_server_entries(server_id, CatalogEntry::mark_unavailable)
    }

    async fn mark_server_tools_available(
        &self,
        _ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()> {
        self.apply_to_server_entries(server_id, CatalogEntry::mark_available)
    }

    async fn find_by_tool_name(
        &self,
        _ctx: &RequestContext,
        tool_name: &str,
    ) -> ToolCatalogResult<Option<CatalogEntry>> {
        let state = self
            .state
            .read()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;
        Ok(state.entries.get(tool_name).cloned())
    }

    async fn list_all(&self, _ctx: &RequestContext) -> ToolCatalogResult<Vec<CatalogEntry>> {
        let state = self
            .state
            .read()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;
        Ok(state.entries.values().cloned().collect())
    }

    async fn record_audit(
        &self,
        _ctx: &RequestContext,
        record: &ToolCallAuditRecord,
    ) -> ToolCatalogResult<()> {
        let mut state = self
            .state
            .write()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))?;
        state.audit_records.push(record.clone());
        Ok(())
    }
}
