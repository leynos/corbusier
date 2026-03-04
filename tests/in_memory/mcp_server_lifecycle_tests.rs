//! In-memory integration tests for MCP server lifecycle management.

use std::sync::Arc;

use corbusier::tool_registry::{
    adapters::{InMemoryMcpServerHost, memory::InMemoryMcpServerRegistry},
    domain::{
        McpServerHealthStatus, McpServerName, McpToolDefinition, McpTransport,
        ToolRegistryDomainError,
    },
    services::{
        McpServerLifecycleService, McpServerLifecycleServiceError, RegisterMcpServerRequest,
    },
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;

type TestService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

struct TestContext {
    host: Arc<InMemoryMcpServerHost>,
    service: TestService,
}

#[fixture]
fn context() -> TestContext {
    let host = Arc::new(InMemoryMcpServerHost::new());
    let service = McpServerLifecycleService::new(
        Arc::new(InMemoryMcpServerRegistry::new()),
        host.clone(),
        Arc::new(DefaultClock),
    );
    TestContext { host, service }
}

fn stdio_request(name: &str) -> Result<RegisterMcpServerRequest, ToolRegistryDomainError> {
    Ok(RegisterMcpServerRequest::new(
        name,
        McpTransport::stdio("mcp-server")?,
    ))
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_start_and_stop_server(context: TestContext) {
    let registered = context
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");

    let started = context
        .service
        .start(registered.id())
        .await
        .expect("start should succeed");
    assert_eq!(started.lifecycle_state().as_str(), "running");

    let stopped = context
        .service
        .stop(registered.id())
        .await
        .expect("stop should succeed");
    assert_eq!(stopped.lifecycle_state().as_str(), "stopped");
    assert_eq!(
        stopped
            .last_health()
            .expect("health snapshot should exist")
            .status(),
        McpServerHealthStatus::Unknown
    );
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_server_name_is_rejected(context: TestContext) {
    context
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("first registration should succeed");

    let result = context
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::Repository(
            corbusier::tool_registry::ports::McpServerRegistryError::DuplicateServerName(_)
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_servers_returns_registered_entries(context: TestContext) {
    context
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");
    context
        .service
        .register(stdio_request("analysis_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");

    let servers = context
        .service
        .list_all()
        .await
        .expect("listing should succeed");

    assert_eq!(servers.len(), 2);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_tools_for_running_server(context: TestContext) {
    let tool = McpToolDefinition::new(
        "search_code",
        "Searches the workspace source tree",
        json!({"type": "object", "properties": {"query": {"type": "string"}}}),
    )
    .expect("tool definition should be valid");
    context
        .host
        .set_tool_catalog(
            McpServerName::new("workspace_tools").expect("valid server name"),
            vec![tool],
        )
        .expect("catalog setup should succeed");

    let registered = context
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");

    context
        .service
        .start(registered.id())
        .await
        .expect("start should succeed");

    let tools = context
        .service
        .list_tools(registered.id())
        .await
        .expect("tool listing should succeed");

    assert_eq!(tools.len(), 1);
    let first = tools.first().expect("tool should exist");
    assert_eq!(first.name(), "search_code");
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_tools_for_stopped_server_is_rejected(context: TestContext) {
    let registered = context
        .service
        .register(stdio_request("workspace_tools").expect("valid test request"))
        .await
        .expect("registration should succeed");

    let result = context.service.list_tools(registered.id()).await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::Domain(
            corbusier::tool_registry::domain::ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
        ))
    ));
}
