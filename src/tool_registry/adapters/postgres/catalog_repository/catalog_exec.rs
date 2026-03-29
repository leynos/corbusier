//! Shared execution helpers for tenant-scoped tool-catalog queries.

use crate::context::TenantId;
use crate::message::adapters::postgres::blocking_helpers::{get_conn_with, run_blocking_with};
use crate::postgres_support::{ensure_tenant_exists, with_tenant_read_tx, with_tenant_tx};
use crate::tool_registry::ports::{ToolCatalogError, ToolCatalogResult};
use diesel::pg::PgConnection;

use super::super::repository::McpServerPgPool;

pub(super) async fn execute_impl<F, T, W>(
    pool: &McpServerPgPool,
    tenant_id: TenantId,
    query_fn: F,
    execute: W,
) -> ToolCatalogResult<T>
where
    F: FnOnce(&mut PgConnection) -> ToolCatalogResult<T> + Send + 'static,
    T: Send + 'static,
    W: FnOnce(&mut PgConnection, TenantId, F) -> ToolCatalogResult<T> + Send + 'static,
{
    let pool_clone = pool.clone();
    run_blocking_with(
        move || {
            let mut conn =
                get_conn_with(&pool_clone, |e| ToolCatalogError::persistence("connect", e))?;
            execute(&mut conn, tenant_id, query_fn)
        },
        |e| ToolCatalogError::persistence("spawn_blocking", e),
    )
    .await
}

pub(super) async fn execute_query<F, T>(
    pool: &McpServerPgPool,
    tenant_id: TenantId,
    query_fn: F,
) -> ToolCatalogResult<T>
where
    F: FnOnce(&mut PgConnection) -> ToolCatalogResult<T> + Send + 'static,
    T: Send + 'static,
{
    execute_impl(pool, tenant_id, query_fn, |conn, tenant, run_query| {
        with_tenant_tx(conn, tenant.into_inner(), run_query)
    })
    .await
}

pub(super) async fn execute_query_with_bootstrap<F, T>(
    pool: &McpServerPgPool,
    tenant_id: TenantId,
    query_fn: F,
) -> ToolCatalogResult<T>
where
    F: FnOnce(&mut PgConnection) -> ToolCatalogResult<T> + Send + 'static,
    T: Send + 'static,
{
    execute_impl(pool, tenant_id, query_fn, |conn, tenant, run_query| {
        with_tenant_tx(conn, tenant.into_inner(), |tx| {
            ensure_tenant_exists(tx, tenant.into_inner())
                .map_err(|e| ToolCatalogError::persistence("ensure_tenant", e))?;
            run_query(tx)
        })
    })
    .await
}

pub(super) async fn execute_read_query<F, T>(
    pool: &McpServerPgPool,
    tenant_id: TenantId,
    query_fn: F,
) -> ToolCatalogResult<T>
where
    F: FnOnce(&mut PgConnection) -> ToolCatalogResult<T> + Send + 'static,
    T: Send + 'static,
{
    execute_impl(pool, tenant_id, query_fn, |conn, tenant, run_query| {
        with_tenant_read_tx(conn, tenant.into_inner(), run_query)
    })
    .await
}
