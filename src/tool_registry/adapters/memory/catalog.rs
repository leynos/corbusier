//! In-memory repository for tool catalog entries and audit records.

use crate::context::{RequestContext, TenantId};
use crate::tool_registry::{
    domain::{CatalogEntry, McpServerId, ToolCallAuditRecord},
    ports::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult},
};
use async_trait::async_trait;
use mockable::DefaultClock;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Thread-safe in-memory tool catalog repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryToolCatalog {
    state: Arc<RwLock<HashMap<TenantId, InMemoryToolCatalogState>>>,
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

    fn read_state(
        &self,
    ) -> ToolCatalogResult<RwLockReadGuard<'_, HashMap<TenantId, InMemoryToolCatalogState>>> {
        self.state
            .read()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))
    }

    fn write_state(
        &self,
    ) -> ToolCatalogResult<RwLockWriteGuard<'_, HashMap<TenantId, InMemoryToolCatalogState>>> {
        self.state
            .write()
            .map_err(|err| ToolCatalogError::persistence(std::io::Error::other(err.to_string())))
    }

    fn apply_to_server_entries(
        &self,
        tenant_id: TenantId,
        server_id: McpServerId,
        apply: fn(&mut CatalogEntry, &DefaultClock),
    ) -> ToolCatalogResult<()> {
        let mut tenants = self.write_state()?;
        let clock = DefaultClock;
        let Some(state) = tenants.get_mut(&tenant_id) else {
            return Ok(());
        };
        for entry in state
            .entries
            .values_mut()
            .filter(|e| e.server_id() == server_id)
        {
            apply(entry, &clock);
        }
        Ok(())
    }

    /// Returns a snapshot of all audit records across all tenants.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError::Persistence`] if the lock is poisoned.
    pub fn audit_records(&self) -> ToolCatalogResult<Vec<ToolCallAuditRecord>> {
        let tenants = self.read_state()?;
        let records = tenants
            .values()
            .flat_map(|s| s.audit_records.iter().cloned())
            .collect();
        Ok(records)
    }
}

#[async_trait]
impl ToolCatalogRepository for InMemoryToolCatalog {
    async fn sync_server_tools(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<()> {
        let mut tenants = self.write_state()?;
        let state = tenants.entry(ctx.tenant_id()).or_default();

        // Stage the new entries, checking for cross-server duplicates
        // within the same tenant before mutating live state.
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
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()> {
        self.apply_to_server_entries(ctx.tenant_id(), server_id, CatalogEntry::mark_unavailable)
    }

    async fn mark_server_tools_available(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()> {
        self.apply_to_server_entries(ctx.tenant_id(), server_id, CatalogEntry::mark_available)
    }

    async fn find_by_tool_name(
        &self,
        ctx: &RequestContext,
        tool_name: &str,
    ) -> ToolCatalogResult<Option<CatalogEntry>> {
        let tenants = self.read_state()?;
        let entry = tenants
            .get(&ctx.tenant_id())
            .and_then(|s| s.entries.get(tool_name).cloned());
        Ok(entry)
    }

    async fn list_all(&self, ctx: &RequestContext) -> ToolCatalogResult<Vec<CatalogEntry>> {
        let tenants = self.read_state()?;
        let entries = tenants
            .get(&ctx.tenant_id())
            .map(|s| s.entries.values().cloned().collect())
            .unwrap_or_default();
        Ok(entries)
    }

    async fn record_audit(
        &self,
        ctx: &RequestContext,
        record: &ToolCallAuditRecord,
    ) -> ToolCatalogResult<()> {
        let mut tenants = self.write_state()?;
        let state = tenants.entry(ctx.tenant_id()).or_default();
        state.audit_records.push(record.clone());
        Ok(())
    }
}
