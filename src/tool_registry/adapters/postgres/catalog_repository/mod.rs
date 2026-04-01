//! `PostgreSQL` repository for tool catalog and audit trail persistence.
//!
//! Tenant context is propagated via `SET LOCAL app.tenant_id`, which sets
//! a `PostgreSQL` session variable scoped to the current transaction.

mod audit_helpers;
mod catalog_exec;
mod mappers;
mod sync_helpers;

use super::{
    catalog_models::{CatalogEntryRow, NewCatalogEntryRow},
    catalog_schema::{mcp_tool_catalog, tool_call_audit_log},
    repository::McpServerPgPool,
};
use crate::context::{RequestContext, TenantId};
use crate::postgres_support::{FromTxError, TxError};
use crate::tool_registry::{
    domain::{CatalogEntry, McpServerId, ToolCallAuditRecord},
    ports::{ToolCatalogError, ToolCatalogRepository, ToolCatalogResult},
};

use async_trait::async_trait;
use diesel::pg::{Pg, PgConnection};
use diesel::prelude::*;
use std::collections::{HashMap, HashSet};

use self::audit_helpers::audit_to_new_row;
use self::catalog_exec::{execute_query, execute_query_with_bootstrap, execute_read_query};
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
        execute_query(&self.pool, tenant_id, move |connection| {
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
        tenant_id: TenantId,
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

    /// Derives a stable advisory-lock key from a tenant UUID.
    ///
    /// Uses SHA-256 (via the `sha2` crate) to ensure the mapping is
    /// deterministic across Rust toolchain versions, unlike
    /// `DefaultHasher` whose output may change between releases.
    fn advisory_lock_key(tenant_id: uuid::Uuid) -> i64 {
        use sha2::{Digest, Sha256};

        let digest: [u8; 32] = Sha256::digest(tenant_id.as_bytes()).into();
        (i64::from(digest[0]) << 56)
            | (i64::from(digest[1]) << 48)
            | (i64::from(digest[2]) << 40)
            | (i64::from(digest[3]) << 32)
            | (i64::from(digest[4]) << 24)
            | (i64::from(digest[5]) << 16)
            | (i64::from(digest[6]) << 8)
            | i64::from(digest[7])
    }

    fn acquire_sync_lock(
        connection: &mut PgConnection,
        tenant_id: TenantId,
    ) -> ToolCatalogResult<()> {
        let tenant_uuid = tenant_id.into_inner();
        diesel::sql_query(format!(
            "SELECT pg_advisory_xact_lock({})",
            Self::advisory_lock_key(tenant_uuid)
        ))
        .execute(connection)
        .map_err(|e| ToolCatalogError::persistence("advisory_lock", e))?;
        Ok(())
    }

    fn sync_rows_tx(
        conn: &mut PgConnection,
        tenant_id: TenantId,
        server_id: McpServerId,
        rows: &[NewCatalogEntryRow],
    ) -> Result<(), diesel::result::Error> {
        let tenant_uuid = tenant_id.into_inner();
        let server_uuid = server_id.into_inner();
        conn.transaction::<_, diesel::result::Error, _>(|transaction| {
            diesel::delete(
                mcp_tool_catalog::table
                    .filter(mcp_tool_catalog::server_id.eq(server_uuid))
                    .filter(mcp_tool_catalog::tenant_id.eq(tenant_uuid)),
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

    async fn run_sync_in_pool(
        &self,
        tenant_id: TenantId,
        server_id: McpServerId,
        rows: Vec<NewCatalogEntryRow>,
    ) -> ToolCatalogResult<()> {
        let candidate_names: HashSet<String> =
            rows.iter().map(|row| row.tool_name.clone()).collect();

        execute_query_with_bootstrap(&self.pool, tenant_id, move |connection| {
            Self::acquire_sync_lock(connection, tenant_id)?;

            let existing_name_counts = sync_helpers::load_conflicting_name_counts(
                connection,
                tenant_id,
                server_id,
                &candidate_names,
            )?;

            if let Some(row) = rows
                .iter()
                .find(|row| existing_name_counts.contains_key(&row.tool_name))
            {
                return Err(sync_helpers::duplicate_entry_error(
                    row,
                    &existing_name_counts,
                ));
            }

            match Self::sync_rows_tx(connection, tenant_id, server_id, &rows) {
                Ok(()) => Ok(()),
                Err(err) => {
                    let attempt = sync_helpers::SyncAttempt {
                        tenant_id,
                        server_id,
                        rows: &rows,
                        candidate_names: &candidate_names,
                    };
                    Err(sync_helpers::map_sync_rows_error(connection, &attempt, err))
                }
            }
        })
        .await
    }

    fn load_entries_for_tenant(
        connection: &mut PgConnection,
        tenant_id: TenantId,
        tool_name: Option<&str>,
    ) -> ToolCatalogResult<Vec<CatalogEntry>> {
        let mut query = mcp_tool_catalog::table
            .filter(mcp_tool_catalog::tenant_id.eq(tenant_id.into_inner()))
            .into_boxed::<Pg>();

        if let Some(name) = tool_name {
            query = query.filter(mcp_tool_catalog::tool_name.eq(name));
        }

        let rows = query
            .select(CatalogEntryRow::as_select())
            .load::<CatalogEntryRow>(connection)
            .map_err(|e| ToolCatalogError::persistence("select", e))?;

        rows.into_iter().map(row_to_entry).collect()
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
        let tenant_id = ctx.tenant_id();
        let rows = Self::to_new_rows(tenant_id, entries)?;
        self.run_sync_in_pool(tenant_id, server_id, rows).await
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
        let name = tool_name.to_owned();
        execute_read_query(&self.pool, tenant_id, move |connection| {
            Self::load_entries_for_tenant(connection, tenant_id, Some(name.as_str()))
        })
        .await
    }

    async fn list_all(&self, ctx: &RequestContext) -> ToolCatalogResult<Vec<CatalogEntry>> {
        let tenant_id = ctx.tenant_id();
        execute_read_query(&self.pool, tenant_id, move |connection| {
            Self::load_entries_for_tenant(connection, tenant_id, None)
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
        execute_query_with_bootstrap(&self.pool, tenant_id, move |connection| {
            diesel::insert_into(tool_call_audit_log::table)
                .values(&row)
                .execute(connection)
                .map_err(|e| ToolCatalogError::persistence("insert", e))?;
            Ok(())
        })
        .await
    }
}
