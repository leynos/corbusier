//! `PostgreSQL` repository for tool catalog and audit trail persistence.
//!
//! Tenant context is propagated via `SET LOCAL app.tenant_id`, which sets
//! a `PostgreSQL` session variable scoped to the current transaction.

mod audit_helpers;
mod mappers;

use super::{
    catalog_models::{CatalogEntryRow, NewCatalogEntryRow},
    catalog_schema::{mcp_tool_catalog, tool_call_audit_log},
    repository::McpServerPgPool,
};
use crate::context::{RequestContext, TenantId};
use crate::message::adapters::postgres::blocking_helpers::{get_conn_with, run_blocking_with};
use crate::message::adapters::postgres::tenant_tx::{FromTxError, TxError, with_tenant_tx};
use crate::tool_registry::{
    domain::{CatalogEntry, CatalogEntryId, McpServerId, ToolCallAuditRecord},
    ports::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult},
};

use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use std::collections::{HashMap, HashSet};

use self::audit_helpers::audit_to_new_row;
use self::mappers::{entry_to_new_row, row_to_entry};

// ---------------------------------------------------------------------------
// Error bridging for the shared transaction helper
// ---------------------------------------------------------------------------

impl FromTxError<Self> for ToolCatalogError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(e) => e,
            TxError::Diesel(e) => Self::persistence("transaction", e),
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
                let mut conn =
                    get_conn_with(&pool, |e| ToolCatalogError::persistence("connect", e))?;
                with_tenant_tx(&mut conn, tenant_id.into_inner(), query_fn)
            },
            |e| ToolCatalogError::persistence("spawn_blocking", e),
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
            .map_err(|e| ToolCatalogError::persistence("update", e))?;
            Ok(())
        })
        .await
    }

    fn validate_same_server(
        server_id: McpServerId,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<()> {
        if let Some(bad) = entries.iter().find(|entry| entry.server_id() != server_id) {
            return Err(ToolCatalogError::MixedServerBatch {
                reason: format!(
                    "entry '{}' belongs to server {} but batch targets {}",
                    bad.tool().name(),
                    bad.server_id(),
                    server_id,
                ),
            });
        }
        Ok(())
    }

    fn find_duplicate_entry(entries: &[CatalogEntry]) -> Option<ToolCatalogError> {
        let name_counts = entries.iter().fold(HashMap::new(), |mut counts, entry| {
            *counts
                .entry(entry.tool().name().to_owned())
                .or_insert(0usize) += 1;
            counts
        });
        let mut seen_names = HashSet::new();
        for entry in entries {
            let tool_name = entry.tool().name().to_owned();
            if !seen_names.insert(tool_name.clone()) {
                return Some(ToolCatalogError::DuplicateWithinBatch {
                    id: entry.id(),
                    tool_name,
                    entry_count: name_counts.get(entry.tool().name()).copied().unwrap_or(2),
                });
            }
        }
        None
    }

    fn to_new_rows(
        tenant_id: uuid::Uuid,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<Vec<NewCatalogEntryRow>> {
        if let Some(err) = Self::find_duplicate_entry(entries) {
            return Err(err);
        }
        Ok(entries
            .iter()
            .map(|entry| entry_to_new_row(entry, tenant_id))
            .collect())
    }

    fn sync_rows_tx(
        conn: &mut PgConnection,
        tenant_id: uuid::Uuid,
        server_id: uuid::Uuid,
        rows: &[NewCatalogEntryRow],
    ) -> Result<(), diesel::result::Error> {
        conn.transaction(|transaction| {
            diesel::delete(
                mcp_tool_catalog::table
                    .filter(mcp_tool_catalog::server_id.eq(server_id))
                    .filter(mcp_tool_catalog::tenant_id.eq(tenant_id)),
            )
            .execute(transaction)?;

            if !rows.is_empty() {
                diesel::insert_into(mcp_tool_catalog::table)
                    .values(rows)
                    .execute(transaction)?;
            }
            Ok(())
        })
    }

    fn load_conflicting_name_counts(
        connection: &mut PgConnection,
        tenant_id: uuid::Uuid,
        server_id: uuid::Uuid,
        candidate_names: &HashSet<String>,
    ) -> ToolCatalogResult<HashMap<String, usize>> {
        let candidate_name_list: Vec<&str> = candidate_names.iter().map(String::as_str).collect();
        mcp_tool_catalog::table
            .filter(mcp_tool_catalog::tenant_id.eq(tenant_id))
            .filter(mcp_tool_catalog::server_id.ne(server_id))
            .filter(mcp_tool_catalog::tool_name.eq_any(candidate_name_list))
            .group_by(mcp_tool_catalog::tool_name)
            .select((mcp_tool_catalog::tool_name, diesel::dsl::count_star()))
            .load::<(String, i64)>(connection)
            .map_err(|e| ToolCatalogError::persistence("select", e))
            .map(|tool_names| {
                tool_names
                    .into_iter()
                    .fold(HashMap::new(), |mut counts, (tool_name, count)| {
                        *counts.entry(tool_name).or_insert(0usize) =
                            usize::try_from(count).unwrap_or_default();
                        counts
                    })
            })
    }

    async fn run_sync_in_pool(
        &self,
        tenant_id: uuid::Uuid,
        server_id: uuid::Uuid,
        rows: Vec<NewCatalogEntryRow>,
    ) -> ToolCatalogResult<()> {
        let tenant = TenantId::from_uuid(tenant_id);
        let candidate_names: HashSet<String> =
            rows.iter().map(|row| row.tool_name.clone()).collect();

        self.execute_query(tenant, move |connection| {
            let existing_name_counts = Self::load_conflicting_name_counts(
                connection,
                tenant_id,
                server_id,
                &candidate_names,
            )?;

            if let Some(row) = rows
                .iter()
                .find(|row| existing_name_counts.contains_key(&row.tool_name))
            {
                return Err(Self::duplicate_entry_error(row, &existing_name_counts));
            }

            match Self::sync_rows_tx(connection, tenant_id, server_id, &rows) {
                Ok(()) => Ok(()),
                Err(err @ DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
                    let refreshed_name_counts = Self::load_conflicting_name_counts(
                        connection,
                        tenant_id,
                        server_id,
                        &candidate_names,
                    )?;

                    rows.iter()
                        .find(|row| refreshed_name_counts.contains_key(&row.tool_name))
                        .map(|row| Self::duplicate_entry_error(row, &refreshed_name_counts))
                        .map_or_else(
                            || Err(ToolCatalogError::persistence("transaction", err)),
                            Err,
                        )
                }
                Err(e) => Err(ToolCatalogError::persistence("transaction", e)),
            }
        })
        .await
    }

    fn duplicate_entry_error(
        row: &NewCatalogEntryRow,
        name_counts: &HashMap<String, usize>,
    ) -> ToolCatalogError {
        let server_count = name_counts.get(&row.tool_name).copied().unwrap_or_default() + 1;
        ToolCatalogError::DuplicateEntry {
            id: CatalogEntryId::from_uuid(row.id),
            tool_name: row.tool_name.clone(),
            server_count,
        }
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
        Self::validate_same_server(server_id, entries)?;
        let tenant_id = ctx.tenant_id().into_inner();
        let rows = Self::to_new_rows(tenant_id, entries)?;
        self.run_sync_in_pool(tenant_id, server_id.into_inner(), rows)
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
    ) -> ToolCatalogResult<Vec<CatalogEntry>> {
        let tenant_id = ctx.tenant_id();
        let tid = tenant_id.into_inner();
        let name = tool_name.to_owned();
        self.execute_query(tenant_id, move |connection| {
            let rows = mcp_tool_catalog::table
                .filter(mcp_tool_catalog::tool_name.eq(&name))
                .filter(mcp_tool_catalog::tenant_id.eq(tid))
                .select(CatalogEntryRow::as_select())
                .load::<CatalogEntryRow>(connection)
                .map_err(|e| ToolCatalogError::persistence("select", e))?;
            rows.into_iter().map(row_to_entry).collect()
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
                .map_err(|e| ToolCatalogError::persistence("select", e))?;
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
                .map_err(|e| ToolCatalogError::persistence("insert", e))?;
            Ok(())
        })
        .await
    }
}
