//! `PostgreSQL` integration tests for tool discovery, catalog persistence,
//! and audit trail.

use std::sync::Arc;

use corbusier::tool_registry::{
    adapters::{
        AllowAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter,
        postgres::{McpServerPgPool, PostgresMcpServerRegistry, PostgresToolCatalog},
    },
    domain::{LogRetentionPolicy, McpServerName, McpToolDefinition, McpTransport, ToolCallRequest},
    services::{
        McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
        ToolDiscoveryRoutingService,
    },
};
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster,
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
            &format!("tool_discovery_{}", Uuid::new_v4()),
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
            policy: Arc::new(AllowAllPolicy),
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

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn catalog_round_trip(
    #[future] context: Result<PgTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;

    ctx.host
        .set_tool_catalog(
            McpServerName::new("workspace_tools").expect("valid name"),
            vec![read_file_tool().expect("valid tool")],
        )
        .expect("catalog setup should succeed");

    let registered = ctx
        .lifecycle
        .register(stdio_request("workspace_tools").expect("valid request"))
        .await
        .expect("registration should succeed");
    ctx.lifecycle
        .start(registered.id())
        .await
        .expect("start should succeed");

    // Discover and persist tools.
    let entries = ctx
        .discovery
        .discover_and_persist_tools(registered.id())
        .await
        .expect("discovery should succeed");
    assert_eq!(entries.len(), 1);

    // Verify via catalog list.
    let catalog_entries = ctx
        .discovery
        .list_catalog()
        .await
        .expect("catalog list should succeed");
    assert_eq!(catalog_entries.len(), 1);
    let entry = catalog_entries.first().expect("first entry should exist");
    assert_eq!(entry.tool().name(), "read_file");
    assert!(entry.available());

    // Mark unavailable and verify.
    ctx.discovery
        .mark_tools_unavailable(registered.id())
        .await
        .expect("mark unavailable should succeed");

    let after_mark = ctx
        .discovery
        .list_catalog()
        .await
        .expect("catalog list should succeed");
    let updated = after_mark.first().expect("first entry should exist");
    assert!(!updated.available());

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn audit_log_persisted(
    #[future] context: Result<PgTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;

    ctx.host
        .set_tool_catalog(
            McpServerName::new("workspace_tools").expect("valid name"),
            vec![read_file_tool().expect("valid tool")],
        )
        .expect("catalog setup should succeed");
    ctx.host
        .set_tool_call_result(
            McpServerName::new("workspace_tools").expect("valid name"),
            "read_file",
            json!({"content": "hello world"}),
        )
        .expect("call result setup should succeed");

    let registered = ctx
        .lifecycle
        .register(stdio_request("workspace_tools").expect("valid request"))
        .await
        .expect("registration should succeed");
    ctx.lifecycle
        .start(registered.id())
        .await
        .expect("start should succeed");
    ctx.discovery
        .discover_and_persist_tools(registered.id())
        .await
        .expect("discovery should succeed");

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = ctx
        .discovery
        .call_tool(&request)
        .await
        .expect("tool call should succeed");

    assert!(result.outcome().is_success());
    assert_eq!(result.tool_name(), "read_file");

    // Verify audit row in PostgreSQL via raw SQL.
    let audit_pool = ctx.pool.clone();
    let audit_count: i64 = tokio::task::spawn_blocking(move || -> Result<i64, BoxError> {
        use diesel::prelude::*;
        let mut conn = audit_pool.get()?;
        let row = diesel::sql_query(
            "SELECT COUNT(*) AS count FROM tool_call_audit_log WHERE tool_name = 'read_file'",
        )
        .get_result::<CountResult>(&mut conn)?;
        Ok(row.count)
    })
    .await
    .expect("spawn_blocking should not panic")?;

    assert_eq!(audit_count, 1);

    Ok(())
}

/// Helper struct for querying count from `PostgreSQL`.
#[derive(diesel::QueryableByName, Debug)]
struct CountResult {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn catalog_survives_service_reconstruction(
    #[future] context: Result<PgTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;

    ctx.host
        .set_tool_catalog(
            McpServerName::new("workspace_tools").expect("valid name"),
            vec![read_file_tool().expect("valid tool")],
        )
        .expect("catalog setup should succeed");

    let registered = ctx
        .lifecycle
        .register(stdio_request("workspace_tools").expect("valid request"))
        .await
        .expect("registration should succeed");
    ctx.lifecycle
        .start(registered.id())
        .await
        .expect("start should succeed");
    ctx.discovery
        .discover_and_persist_tools(registered.id())
        .await
        .expect("discovery should succeed");

    // Construct a new discovery service instance from the same pool.
    let registry2 = Arc::new(PostgresMcpServerRegistry::new(ctx.pool.clone()));
    let catalog2 = Arc::new(PostgresToolCatalog::new(ctx.pool.clone()));
    let clock2 = Arc::new(DefaultClock);
    let discovery2: TestDiscoveryService = ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: catalog2,
            registry: registry2,
            host: ctx.host.clone(),
            policy: Arc::new(AllowAllPolicy),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock2,
    );

    // The new service instance should see the catalog entries persisted by the first.
    let entries = discovery2
        .list_catalog()
        .await
        .expect("catalog list should succeed");
    assert_eq!(entries.len(), 1);
    let first = entries.first().expect("first entry should exist");
    assert_eq!(first.tool().name(), "read_file");

    Ok(())
}
