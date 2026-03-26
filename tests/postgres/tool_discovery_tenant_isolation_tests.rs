//! `PostgreSQL` integration tests for tenant isolation in tool discovery.

use std::sync::Arc;

use corbusier::tool_registry::{
    adapters::{
        AllowAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter, StubGovernance,
        postgres::{McpServerPgPool, PostgresMcpServerRegistry, PostgresToolCatalog},
    },
    domain::{LogRetentionPolicy, McpServerName, McpToolDefinition, McpTransport},
    services::{
        McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
        ToolDiscoveryRoutingService,
    },
};
use diesel::PgConnection;
use diesel::RunQueryDsl;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, other_tenant_ctx, postgres_cluster,
    test_request_ctx,
};

type TestLifecycleService =
    McpServerLifecycleService<PostgresMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

type TestDiscoveryService = ToolDiscoveryRoutingService<
    PostgresToolCatalog,
    PostgresMcpServerRegistry,
    InMemoryMcpServerHost,
    AllowAllPolicy,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

struct PgTestContext {
    host: Arc<InMemoryMcpServerHost>,
    lifecycle: TestLifecycleService,
    discovery: TestDiscoveryService,
    pool: McpServerPgPool,
    _temp_db: TemporaryDatabase,
}

async fn setup_context(cluster: PostgresCluster) -> Result<PgTestContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(
            &format!("tool_discovery_tenant_{}", Uuid::new_v4()),
            TEMPLATE_DB,
        )
        .await?;

    let manager = ConnectionManager::<PgConnection>::new(db.url());
    let pool: McpServerPgPool = diesel::r2d2::Pool::builder()
        .max_size(2)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)?;

    let registry = Arc::new(PostgresMcpServerRegistry::new(pool.clone()));
    let host = Arc::new(InMemoryMcpServerHost::new());
    let catalog = Arc::new(PostgresToolCatalog::new(pool.clone()));
    let clock = Arc::new(DefaultClock);

    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let discovery = ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog,
            registry,
            host: host.clone(),
            policy: Arc::new(StubGovernance::allowing()),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock,
    );

    Ok(PgTestContext {
        host,
        lifecycle,
        discovery,
        pool,
        _temp_db: db,
    })
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<PgTestContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_context(cluster).await
}

fn stdio_request(
    name: &str,
) -> Result<RegisterMcpServerRequest, corbusier::tool_registry::domain::ToolRegistryDomainError> {
    Ok(RegisterMcpServerRequest::new(
        name,
        McpTransport::stdio("mcp-server")?,
    ))
}

fn read_file_tool() -> Result<McpToolDefinition, eyre::Report> {
    Ok(McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    )?)
}

#[derive(diesel::QueryableByName, Debug)]
struct PlanRow {
    #[diesel(column_name = "QUERY PLAN")]
    #[diesel(sql_type = diesel::sql_types::Text)]
    plan: String,
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn composite_fk_rejects_cross_tenant_server_reference(
    #[future] context: Result<PgTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let tenant_a = test_request_ctx();
    let tenant_b = other_tenant_ctx(&tenant_a);

    let server = ctx
        .lifecycle
        .register(
            &tenant_a,
            stdio_request("workspace_tools").expect("valid request"),
        )
        .await
        .expect("registration should succeed");

    let pool = ctx.pool.clone();
    let tenant_b_id = tenant_b.tenant_id().into_inner();
    let server_id = server.id().into_inner();
    let result = tokio::task::spawn_blocking(move || -> Result<(), BoxError> {
        let mut conn = pool.get()?;
        diesel::sql_query(concat!(
            "INSERT INTO mcp_tool_catalog (",
            "id, tenant_id, server_id, server_name, tool_name, tool_description, ",
            "input_schema, output_schema, available, discovered_at, updated_at",
            ") VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, NULL, true, NOW(), NOW())"
        ))
        .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
        .bind::<diesel::sql_types::Uuid, _>(tenant_b_id)
        .bind::<diesel::sql_types::Uuid, _>(server_id)
        .bind::<diesel::sql_types::Text, _>("workspace_tools")
        .bind::<diesel::sql_types::Text, _>("read_file")
        .bind::<diesel::sql_types::Text, _>("Reads a file")
        .bind::<diesel::sql_types::Text, _>(r#"{"type":"object"}"#)
        .execute(&mut conn)?;
        Ok(())
    })
    .await
    .expect("spawn_blocking should not panic");

    assert!(result.is_err(), "cross-tenant insert should violate FK");
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn explain_uses_tenant_server_index_for_catalog_lookup(
    #[future] context: Result<PgTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let request_ctx = test_request_ctx();

    ctx.host
        .set_tool_catalog(
            McpServerName::new("workspace_tools").expect("valid name"),
            vec![read_file_tool().expect("valid tool")],
        )
        .expect("catalog setup should succeed");

    let registered = ctx
        .lifecycle
        .register(
            &request_ctx,
            stdio_request("workspace_tools").expect("valid request"),
        )
        .await
        .expect("registration should succeed");
    ctx.lifecycle
        .start(&request_ctx, registered.id())
        .await
        .expect("start should succeed");
    ctx.discovery
        .discover_and_persist_tools(&request_ctx, registered.id())
        .await
        .expect("discovery should succeed");

    let pool = ctx.pool.clone();
    let tenant_id = request_ctx.tenant_id().into_inner();
    let server_id = registered.id().into_inner();
    let plan_rows = tokio::task::spawn_blocking(move || -> Result<Vec<PlanRow>, BoxError> {
        let mut conn = pool.get()?;
        diesel::sql_query("SET enable_seqscan = off").execute(&mut conn)?;
        let rows = diesel::sql_query(concat!(
            "EXPLAIN (FORMAT TEXT) ",
            "SELECT * FROM mcp_tool_catalog ",
            "WHERE tenant_id = $1 AND server_id = $2"
        ))
        .bind::<diesel::sql_types::Uuid, _>(tenant_id)
        .bind::<diesel::sql_types::Uuid, _>(server_id)
        .load::<PlanRow>(&mut conn)?;
        Ok(rows)
    })
    .await
    .expect("spawn_blocking should not panic")?;

    let plan_text = plan_rows
        .iter()
        .map(|row| row.plan.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        plan_text.contains("idx_mcp_tool_catalog_server_tenant"),
        "expected composite index in plan, got:\n{plan_text}"
    );

    Ok(())
}
