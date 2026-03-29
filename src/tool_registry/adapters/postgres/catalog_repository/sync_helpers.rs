//! Sync-specific duplicate-detection helpers for the tool catalog adapter.

use std::collections::{HashMap, HashSet};

use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorInformation, DatabaseErrorKind, Error as DieselError};

use crate::tool_registry::{
    adapters::postgres::{catalog_models::NewCatalogEntryRow, catalog_schema::mcp_tool_catalog},
    domain::CatalogEntryId,
    ports::{ToolCatalogError, ToolCatalogResult},
};

pub(super) struct SyncAttempt<'a> {
    pub(super) tenant_id: uuid::Uuid,
    pub(super) server_id: uuid::Uuid,
    pub(super) rows: &'a [NewCatalogEntryRow],
    pub(super) candidate_names: &'a HashSet<String>,
}

pub(super) fn load_conflicting_name_counts(
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

fn is_catalog_name_unique_violation(info: &dyn DatabaseErrorInformation) -> bool {
    info.constraint_name()
        .is_some_and(|name| name == "idx_mcp_tool_catalog_tenant_tool_name")
}

fn duplicate_entry_from_counts(
    rows: &[NewCatalogEntryRow],
    name_counts: &HashMap<String, usize>,
) -> Option<ToolCatalogError> {
    rows.iter()
        .find(|row| name_counts.contains_key(&row.tool_name))
        .map(|row| duplicate_entry_error(row, name_counts))
}

pub(super) fn map_sync_rows_error(
    connection: &mut PgConnection,
    attempt: &SyncAttempt<'_>,
    err: DieselError,
) -> ToolCatalogError {
    if let DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, ref info) = err
        && is_catalog_name_unique_violation(info.as_ref())
        && let Ok(refreshed_counts) = load_conflicting_name_counts(
            connection,
            attempt.tenant_id,
            attempt.server_id,
            attempt.candidate_names,
        )
        && let Some(duplicate_entry) = duplicate_entry_from_counts(attempt.rows, &refreshed_counts)
    {
        return duplicate_entry;
    }

    ToolCatalogError::persistence("transaction", err)
}

pub(super) fn duplicate_entry_error(
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
