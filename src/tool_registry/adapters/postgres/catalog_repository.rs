//! `PostgreSQL` repository for tool catalog and audit trail persistence.

use super::{
    catalog_models::{CatalogEntryRow, NewAuditLogRow, NewCatalogEntryRow},
    catalog_schema::{mcp_tool_catalog, tool_call_audit_log},
    repository::McpServerPgPool,
};
use crate::tool_registry::{
    domain::{
        CatalogEntry, CatalogEntryId, McpServerId, McpServerName, McpToolDefinition,
        PersistedCatalogEntryData, ToolCallAuditRecord, ToolCallOutcome,
    },
    ports::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult},
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

/// `PostgreSQL`-backed repository for tool catalog entries and audit records.
#[derive(Debug, Clone)]
pub struct PostgresToolCatalog {
    pool: McpServerPgPool,
}

impl PostgresToolCatalog {
    /// Creates a new catalog repository from a `PostgreSQL` pool.
    #[must_use]
    pub const fn new(pool: McpServerPgPool) -> Self {
        Self { pool }
    }

    /// Sets the `available` flag for every catalog entry belonging to
    /// `server_id`.
    async fn set_server_tools_availability(
        &self,
        server_id: McpServerId,
        available: bool,
    ) -> ToolCatalogResult<()> {
        let sid = server_id.into_inner();
        self.run_blocking(move |connection| {
            diesel::update(mcp_tool_catalog::table.filter(mcp_tool_catalog::server_id.eq(sid)))
                .set(mcp_tool_catalog::available.eq(available))
                .execute(connection)
                .map_err(ToolCatalogError::persistence)?;
            Ok(())
        })
        .await
    }

    async fn run_blocking<F, T>(&self, operation: F) -> ToolCatalogResult<T>
    where
        F: FnOnce(&mut PgConnection) -> ToolCatalogResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool.get().map_err(ToolCatalogError::persistence)?;
            operation(&mut connection)
        })
        .await
        .map_err(ToolCatalogError::persistence)?
    }
}

#[async_trait]
impl ToolCatalogRepository for PostgresToolCatalog {
    async fn sync_server_tools(
        &self,
        server_id: McpServerId,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<()> {
        let sid = server_id.into_inner();
        let rows: Vec<NewCatalogEntryRow> = entries.iter().map(entry_to_new_row).collect();

        self.run_blocking(move |connection| {
            // Delete existing entries for this server, then insert new ones.
            diesel::delete(mcp_tool_catalog::table.filter(mcp_tool_catalog::server_id.eq(sid)))
                .execute(connection)
                .map_err(ToolCatalogError::persistence)?;

            for row in &rows {
                diesel::insert_into(mcp_tool_catalog::table)
                    .values(row)
                    .execute(connection)
                    .map_err(|err| match err {
                        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                            ToolCatalogError::DuplicateEntry(CatalogEntryId::from_uuid(row.id))
                        }
                        other => ToolCatalogError::persistence(other),
                    })?;
            }

            Ok(())
        })
        .await
    }

    async fn mark_server_tools_unavailable(&self, server_id: McpServerId) -> ToolCatalogResult<()> {
        self.set_server_tools_availability(server_id, false).await
    }

    async fn mark_server_tools_available(&self, server_id: McpServerId) -> ToolCatalogResult<()> {
        self.set_server_tools_availability(server_id, true).await
    }

    async fn find_by_tool_name(&self, tool_name: &str) -> ToolCatalogResult<Option<CatalogEntry>> {
        let name = tool_name.to_owned();
        self.run_blocking(move |connection| {
            let row = mcp_tool_catalog::table
                .filter(mcp_tool_catalog::tool_name.eq(&name))
                .select(CatalogEntryRow::as_select())
                .first::<CatalogEntryRow>(connection)
                .optional()
                .map_err(ToolCatalogError::persistence)?;
            row.map(row_to_entry).transpose()
        })
        .await
    }

    async fn list_all(&self) -> ToolCatalogResult<Vec<CatalogEntry>> {
        self.run_blocking(move |connection| {
            let rows = mcp_tool_catalog::table
                .select(CatalogEntryRow::as_select())
                .load::<CatalogEntryRow>(connection)
                .map_err(ToolCatalogError::persistence)?;
            rows.into_iter().map(row_to_entry).collect()
        })
        .await
    }

    async fn record_audit(&self, record: &ToolCallAuditRecord) -> ToolCatalogResult<()> {
        let row = audit_to_new_row(record);
        self.run_blocking(move |connection| {
            diesel::insert_into(tool_call_audit_log::table)
                .values(&row)
                .execute(connection)
                .map_err(ToolCatalogError::persistence)?;
            Ok(())
        })
        .await
    }
}

fn entry_to_new_row(entry: &CatalogEntry) -> NewCatalogEntryRow {
    let tool = entry.tool();
    NewCatalogEntryRow {
        id: entry.id().into_inner(),
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

fn row_to_entry(row: CatalogEntryRow) -> ToolCatalogResult<CatalogEntry> {
    let server_name = McpServerName::new(&row.server_name)
        .map_err(|err| ToolCatalogError::InvalidPersistedData(std::sync::Arc::new(err)))?;
    let mut tool = McpToolDefinition::new(row.tool_name, row.tool_description, row.input_schema)
        .map_err(|err| ToolCatalogError::InvalidPersistedData(std::sync::Arc::new(err)))?;
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

#[expect(
    clippy::cast_possible_truncation,
    reason = "duration_ms is always positive and within i64 range for tool calls"
)]
fn audit_to_new_row(record: &ToolCallAuditRecord) -> NewAuditLogRow {
    let (outcome_str, outcome_content, outcome_error) = match record.outcome() {
        ToolCallOutcome::Success { content } => ("success".to_owned(), Some(content.clone()), None),
        ToolCallOutcome::Failure { error } => ("failure".to_owned(), None, Some(error.clone())),
    };

    NewAuditLogRow {
        id: record.id(),
        call_id: record.call_id().into_inner(),
        tool_name: record.tool_name().to_owned(),
        server_id: record.server_id().into_inner(),
        parameters: record.parameters().clone(),
        outcome: outcome_str,
        outcome_content,
        outcome_error,
        duration_ms: record.duration().as_millis() as i64,
        initiated_at: record.initiated_at(),
        completed_at: record.completed_at(),
        stderr_log_path: record.stderr_log_path().map(str::to_owned),
    }
}
