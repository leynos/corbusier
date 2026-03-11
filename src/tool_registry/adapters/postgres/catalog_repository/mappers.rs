//! Row/domain mapping helpers for the tool catalog `PostgreSQL` adapter.

use crate::tool_registry::{
    adapters::postgres::catalog_models::{CatalogEntryRow, NewCatalogEntryRow},
    domain::{
        CatalogEntry, CatalogEntryId, McpServerId, McpServerName, McpToolDefinition,
        PersistedCatalogEntryData,
    },
    ports::{ToolCatalogError, ToolCatalogResult},
};

/// Maps a domain catalog entry into an insertable row.
pub(super) fn entry_to_new_row(entry: &CatalogEntry, tenant_id: uuid::Uuid) -> NewCatalogEntryRow {
    let tool = entry.tool();
    NewCatalogEntryRow {
        id: entry.id().into_inner(),
        tenant_id,
        server_id: entry.server_id().into_inner(),
        server_name: entry.server_name().as_str().to_owned(),
        tool_name: tool.name().to_owned(),
        tool_description: tool.description().to_owned(),
        input_schema: tool.input_schema().clone(),
        output_schema: tool.output_schema().cloned(),
        available: entry.available(),
        discovered_at: entry.discovered_at(),
        updated_at: entry.updated_at(),
    }
}

/// Reconstructs a domain catalog entry from a persisted row.
pub(super) fn row_to_entry(row: CatalogEntryRow) -> ToolCatalogResult<CatalogEntry> {
    let server_name = McpServerName::new(&row.server_name)
        .map_err(|e| ToolCatalogError::invalid_persisted_data("server_name", e))?;
    let mut tool = McpToolDefinition::new(row.tool_name, row.tool_description, row.input_schema)
        .map_err(|e| ToolCatalogError::invalid_persisted_data("tool_definition", e))?;
    if let Some(output) = row.output_schema {
        tool = tool.with_output_schema(output);
    }

    Ok(CatalogEntry::from_persisted(PersistedCatalogEntryData {
        id: CatalogEntryId::from_uuid(row.id),
        server_id: McpServerId::from_uuid(row.server_id),
        server_name,
        tool,
        available: row.available,
        discovered_at: row.discovered_at,
        updated_at: row.updated_at,
    }))
}
