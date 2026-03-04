//! `PostgreSQL` integration tests for MCP server lifecycle management.

use std::sync::Arc;

use corbusier::tool_registry::{
    adapters::{
        InMemoryMcpServerHost,
        postgres::{McpServerPgPool, PostgresMcpServerRegistry},
    },
    domain::{McpServerHealthStatus, McpServerName, McpToolDefinition, McpTransport},
    services::{
        McpServerLifecycleService, McpServerLifecycleServiceError, RegisterMcpServerRequest,
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

type TestService =
    McpServerLifecycleService<PostgresMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

struct McpServerTestContext {
    host: Arc<InMemoryMcpServerHost>,
    service: TestService,
    _temp_db: TemporaryDatabase,
}

async fn setup_context(cluster: PostgresCluster) -> Result<McpServerTestContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("mcp_servers_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;

    let manager = ConnectionManager::<PgConnection>::new(db.url());
    let pool: McpServerPgPool = diesel::r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)?;

    let host = Arc::new(InMemoryMcpServerHost::new());
    let service = McpServerLifecycleService::new(
        Arc::new(PostgresMcpServerRegistry::new(pool)),
        host.clone(),
        Arc::new(DefaultClock),
    );

    Ok(McpServerTestContext {
        host,
        service,
        _temp_db: db,
    })
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<McpServerTestContext, BoxError> {
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

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_register_and_find_by_name(
    #[future] context: Result<McpServerTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;

    let created = ctx
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");

    let found = ctx
        .service
        .find_by_name("workspace_tools")
        .await
        .expect("lookup should succeed")
        .expect("server should exist");

    assert_eq!(found.id(), created.id());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_start_persists_running_state(
    #[future] context: Result<McpServerTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let created = ctx
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");

    ctx.service
        .start(created.id())
        .await
        .expect("start should succeed");

    let found = ctx
        .service
        .find_by_name("workspace_tools")
        .await
        .expect("lookup should succeed")
        .expect("server should exist");

    assert_eq!(found.lifecycle_state().as_str(), "running");
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_refresh_health_persists_unhealthy_status(
    #[future] context: Result<McpServerTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let created = ctx
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");

    ctx.service
        .start(created.id())
        .await
        .expect("start should succeed");
    ctx.host
        .set_unhealthy(created.id(), "backend timed out")
        .expect("host should accept unhealthy marker");

    let refreshed = ctx
        .service
        .refresh_health(created.id())
        .await
        .expect("health refresh should succeed");

    assert_eq!(
        refreshed
            .last_health()
            .expect("health snapshot should exist")
            .status(),
        McpServerHealthStatus::Unhealthy
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_duplicate_name_is_rejected(
    #[future] context: Result<McpServerTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;

    ctx.service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("first registration should succeed");

    let result = ctx
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::Repository(
            corbusier::tool_registry::ports::McpServerRegistryError::DuplicateServerName(_)
        ))
    ));

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_list_tools_for_running_server(
    #[future] context: Result<McpServerTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let tool = McpToolDefinition::new(
        "search_code",
        "Searches the workspace source tree",
        json!({"type": "object", "properties": {"query": {"type": "string"}}}),
    )
    .expect("tool definition should be valid");
    ctx.host
        .set_tool_catalog(
            McpServerName::new("workspace_tools").expect("valid name"),
            vec![tool],
        )
        .expect("catalog should be set");

    let created = ctx
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");

    ctx.service
        .start(created.id())
        .await
        .expect("start should succeed");

    let tools = ctx
        .service
        .list_tools(created.id())
        .await
        .expect("tool listing should succeed");

    assert_eq!(tools.len(), 1);
    Ok(())
}
