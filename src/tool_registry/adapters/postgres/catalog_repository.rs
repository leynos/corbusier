//! `PostgreSQL` repository for tool catalog and audit trail persistence.
//!
//! Tenant context is propagated via `SET LOCAL app.tenant_id`, which sets
//! a `PostgreSQL` session variable scoped to the current transaction.

use super::{
    catalog_models::{CatalogEntryRow, NewAuditLogRow, NewCatalogEntryRow},
    catalog_schema::{mcp_tool_catalog, tool_call_audit_log},
    repository::McpServerPgPool,
};
use crate::context::{RequestContext, TenantId};
use crate::message::adapters::postgres::blocking_helpers::{get_conn_with, run_blocking_with};
use crate::message::adapters::postgres::tenant_tx::{FromTxError, TxError, with_tenant_tx};
use crate::tool_registry::{
    domain::{
        CatalogEntry, CatalogEntryId, McpServerId, McpServerName, McpToolDefinition,
        PersistedCatalogEntryData, ToolCallAuditRecord, ToolCallOutcome, redact_outcome_content,
        redact_parameters,
    },
    ports::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult},
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};

// ---------------------------------------------------------------------------
// Error bridging for the shared transaction helper
// ---------------------------------------------------------------------------

impl FromTxError<Self> for ToolCatalogError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(e) => e,
            TxError::Diesel(e) => Self::persistence(e),
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter
// ---------------------------------------------------------------------------

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

    /// Executes a query inside a transaction with tenant context.
    async fn execute_query<F, T>(&self, tenant_id: TenantId, query_fn: F) -> ToolCatalogResult<T>
    where
        F: FnOnce(&mut PgConnection) -> ToolCatalogResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        run_blocking_with(
            move || {
                let mut conn = get_conn_with(&pool, ToolCatalogError::persistence)?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), query_fn)
            },
            ToolCatalogError::persistence,
        )
        .await
    }

    /// Sets the `available` flag and refreshes `updated_at` for every
    /// catalog entry belonging to `server_id` within the tenant scope.
    async fn set_server_tools_availability(
        &self,
        tenant_id: TenantId,
        server_id: McpServerId,
        available: bool,
    ) -> ToolCatalogResult<()> {
        let sid = server_id.into_inner();
        let tid = tenant_id.into_inner();
        self.execute_query(tenant_id, move |connection| {
            diesel::update(
                mcp_tool_catalog::table
                    .filter(mcp_tool_catalog::server_id.eq(sid))
                    .filter(mcp_tool_catalog::tenant_id.eq(tid)),
            )
            .set((
                mcp_tool_catalog::available.eq(available),
                mcp_tool_catalog::updated_at.eq(diesel::dsl::now),
            ))
            .execute(connection)
            .map_err(ToolCatalogError::persistence)?;
            Ok(())
        })
        .await
    }
}

#[async_trait]
impl ToolCatalogRepository for PostgresToolCatalog {
    async fn sync_server_tools(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<()> {
        let tenant_id = ctx.tenant_id();
        let sid = server_id.into_inner();
        let tid = tenant_id.into_inner();
        let rows: Vec<NewCatalogEntryRow> =
            entries.iter().map(|e| entry_to_new_row(e, tid)).collect();

        self.execute_query(tenant_id, move |connection| {
            diesel::delete(
                mcp_tool_catalog::table
                    .filter(mcp_tool_catalog::server_id.eq(sid))
                    .filter(mcp_tool_catalog::tenant_id.eq(tid)),
            )
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

    async fn mark_server_tools_unavailable(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()> {
        self.set_server_tools_availability(ctx.tenant_id(), server_id, false)
            .await
    }

    async fn mark_server_tools_available(
        &self,
        ctx: &RequestContext,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()> {
        self.set_server_tools_availability(ctx.tenant_id(), server_id, true)
            .await
    }

    async fn find_by_tool_name(
        &self,
        ctx: &RequestContext,
        tool_name: &str,
    ) -> ToolCatalogResult<Option<CatalogEntry>> {
        let tenant_id = ctx.tenant_id();
        let tid = tenant_id.into_inner();
        let name = tool_name.to_owned();
        self.execute_query(tenant_id, move |connection| {
            let row = mcp_tool_catalog::table
                .filter(mcp_tool_catalog::tool_name.eq(&name))
                .filter(mcp_tool_catalog::tenant_id.eq(tid))
                .select(CatalogEntryRow::as_select())
                .first::<CatalogEntryRow>(connection)
                .optional()
                .map_err(ToolCatalogError::persistence)?;
            row.map(row_to_entry).transpose()
        })
        .await
    }

    async fn list_all(&self, ctx: &RequestContext) -> ToolCatalogResult<Vec<CatalogEntry>> {
        let tenant_id = ctx.tenant_id();
        let tid = tenant_id.into_inner();
        self.execute_query(tenant_id, move |connection| {
            let rows = mcp_tool_catalog::table
                .filter(mcp_tool_catalog::tenant_id.eq(tid))
                .select(CatalogEntryRow::as_select())
                .load::<CatalogEntryRow>(connection)
                .map_err(ToolCatalogError::persistence)?;
            rows.into_iter().map(row_to_entry).collect()
        })
        .await
    }

    async fn record_audit(
        &self,
        ctx: &RequestContext,
        record: &ToolCallAuditRecord,
    ) -> ToolCatalogResult<()> {
        let tenant_id = ctx.tenant_id();
        let tid = tenant_id.into_inner();
        let row = audit_to_new_row(record, tid);
        self.execute_query(tenant_id, move |connection| {
            diesel::insert_into(tool_call_audit_log::table)
                .values(&row)
                .execute(connection)
                .map_err(ToolCatalogError::persistence)?;
            Ok(())
        })
        .await
    }
}

fn entry_to_new_row(entry: &CatalogEntry, tenant_id: uuid::Uuid) -> NewCatalogEntryRow {
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
fn audit_to_new_row(record: &ToolCallAuditRecord, tenant_id: uuid::Uuid) -> NewAuditLogRow {
    let (outcome_str, outcome_content, outcome_error) = match record.outcome() {
        ToolCallOutcome::Success { content } => (
            "success".to_owned(),
            Some(redact_outcome_content(content)),
            None,
        ),
        ToolCallOutcome::Failure { error } => ("failure".to_owned(), None, Some(error.clone())),
    };

    NewAuditLogRow {
        id: record.id(),
        tenant_id,
        call_id: record.call_id().into_inner(),
        tool_name: record.tool_name().to_owned(),
        server_id: record.server_id().into_inner(),
        parameters: redact_parameters(record.parameters()),
        outcome: outcome_str,
        outcome_content,
        outcome_error,
        duration_ms: record.duration().as_millis() as i64,
        initiated_at: record.initiated_at(),
        completed_at: record.completed_at(),
        stderr_log_path: record.stderr_log_path().map(str::to_owned),
    }
}
