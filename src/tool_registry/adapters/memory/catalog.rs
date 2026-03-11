//! In-memory repository for tool catalog entries and audit records.

use crate::context::{RequestContext, TenantId};
use crate::tool_registry::{
    domain::{CatalogEntry, CatalogEntryId, McpServerId, ToolCallAuditRecord},
    ports::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult},
};
use async_trait::async_trait;
use mockable::DefaultClock;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Thread-safe in-memory tool catalog repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryToolCatalog {
    state: Arc<RwLock<HashMap<TenantId, InMemoryToolCatalogState>>>,
}

#[derive(Debug, Default)]
struct InMemoryToolCatalogState {
    entries: HashMap<CatalogEntryId, CatalogEntry>,
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
            .map_err(|err| ToolCatalogError::persistence("read_lock", err))
    }

    fn write_state(
        &self,
    ) -> ToolCatalogResult<RwLockWriteGuard<'_, HashMap<TenantId, InMemoryToolCatalogState>>> {
        self.state
            .write()
            .map_err(|err| ToolCatalogError::persistence("write_lock", err))
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

    /// Returns a snapshot of audit records for a specific tenant.
    ///
    /// # Errors
    ///
    /// Returns [`ToolCatalogError::Persistence`] if the lock is poisoned.
    pub fn audit_records(
        &self,
        tenant_id: TenantId,
    ) -> ToolCatalogResult<Vec<ToolCallAuditRecord>> {
        let tenants = self.read_state()?;
        let records = tenants
            .get(&tenant_id)
            .map(|s| s.audit_records.clone())
            .unwrap_or_default();
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
        if let Some(bad) = entries.iter().find(|e| e.server_id() != server_id) {
            return Err(ToolCatalogError::MixedServerBatch {
                reason: format!(
                    "entry '{}' belongs to server {} but batch targets {}",
                    bad.tool().name(),
                    bad.server_id(),
                    server_id,
                ),
            });
        }
        let mut tenants = self.write_state()?;
        let state = tenants.entry(ctx.tenant_id()).or_default();

        // Stage the new entries, rejecting within-batch and cross-server duplicates.
        let existing_name_counts = state
            .entries
            .values()
            .filter(|e| e.server_id() != server_id)
            .fold(HashMap::new(), |mut counts, entry| {
                *counts
                    .entry(entry.tool().name().to_owned())
                    .or_insert(0usize) += 1;
                counts
            });
        let mut staged: HashMap<CatalogEntryId, CatalogEntry> = HashMap::new();
        let mut seen_names: HashSet<String> = HashSet::new();
        for entry in entries {
            let tool_name = entry.tool().name().to_owned();
            if !seen_names.insert(tool_name.clone()) {
                return Err(ToolCatalogError::DuplicateWithinBatch {
                    id: entry.id(),
                    tool_name,
                    entry_count: 2,
                });
            }
            if let Some(existing_count) = existing_name_counts.get(&tool_name) {
                return Err(ToolCatalogError::DuplicateEntry {
                    id: entry.id(),
                    tool_name,
                    server_count: existing_count + 1,
                });
            }
            staged.insert(entry.id(), entry.clone());
        }

        // Remove previous entries for this server, then insert new ones.
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
    ) -> ToolCatalogResult<Vec<CatalogEntry>> {
        let tenants = self.read_state()?;
        let entries = tenants
            .get(&ctx.tenant_id())
            .map(|s| {
                s.entries
                    .values()
                    .filter(|e| e.tool().name() == tool_name)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        Ok(entries)
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

#[cfg(test)]
mod tests {
    use super::InMemoryToolCatalog;
    use crate::test_support::test_request_ctx;
    use crate::tool_registry::{
        domain::{CatalogEntry, McpServerId, McpServerName, McpToolDefinition},
        ports::{ToolCatalogError, ToolCatalogRepository},
    };
    use mockable::DefaultClock;
    use serde_json::json;

    fn catalog_entry(
        server_id: McpServerId,
        server_name: &str,
        tool_name: &str,
    ) -> Result<CatalogEntry, eyre::Report> {
        Ok(CatalogEntry::new(
            server_id,
            McpServerName::new(server_name)?,
            McpToolDefinition::new(
                tool_name,
                "Reads a file from the workspace",
                json!({"type": "object", "properties": {"path": {"type": "string"}}}),
            )?,
            &DefaultClock,
        ))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn sync_server_tools_rejects_duplicate_name_from_another_server() {
        let catalog = InMemoryToolCatalog::new();
        let ctx = test_request_ctx();
        let first_server = McpServerId::new();
        let second_server = McpServerId::new();

        catalog
            .sync_server_tools(
                &ctx,
                first_server,
                &[catalog_entry(first_server, "file_tools", "read_file")
                    .expect("first catalog entry should be valid")],
            )
            .await
            .expect("first sync should succeed");

        let result = catalog
            .sync_server_tools(
                &ctx,
                second_server,
                &[catalog_entry(second_server, "backup_tools", "read_file")
                    .expect("second catalog entry should be valid")],
            )
            .await;

        assert!(matches!(
            result,
            Err(ToolCatalogError::DuplicateEntry {
                tool_name,
                server_count: 2,
                ..
            }) if tool_name == "read_file"
        ));
    }
}
