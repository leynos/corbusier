//! Unit tests for MCP server lifecycle orchestration.

use super::{McpServerLifecycleService, McpServerLifecycleServiceError, RegisterMcpServerRequest};
use crate::{
    context::RequestContext,
    test_support::HealthProbeFailureHost,
    tool_registry::{
        adapters::{InMemoryMcpServerHost, memory::InMemoryMcpServerRegistry},
        domain::{
            McpServerHealthStatus, McpServerId, McpServerLifecycleState, McpServerName,
            McpToolDefinition, McpTransport, ToolRegistryDomainError,
        },
    },
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use std::sync::Arc;

type TestService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

#[derive(Debug, Clone, Copy)]
enum LifecycleScenario {
    StartAndList,
    StopBlocksQueries,
    RefreshHealth,
    StartStopStart,
}

use crate::test_support::test_request_ctx;

#[fixture]
fn service_bundle() -> (Arc<InMemoryMcpServerHost>, TestService) {
    let host = Arc::new(InMemoryMcpServerHost::new());
    let service = McpServerLifecycleService::new(
        Arc::new(InMemoryMcpServerRegistry::new()),
        host.clone(),
        Arc::new(DefaultClock),
    );
    (host, service)
}

fn stdio_request(name: &str) -> Result<RegisterMcpServerRequest, ToolRegistryDomainError> {
    let transport = McpTransport::stdio("mcp-server")?;
    Ok(RegisterMcpServerRequest::new(name, transport))
}

/// Creates a standard `read_file` tool definition.
fn create_read_file_tool() -> Result<McpToolDefinition> {
    Ok(McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "properties": {"path": {"type": "string"}}}),
    )?)
}

/// Sets up a tool catalog for the given server on the host.
fn setup_tool_catalog(
    host: &Arc<InMemoryMcpServerHost>,
    server_name: &str,
    tools: Vec<McpToolDefinition>,
) -> Result<()> {
    host.set_tool_catalog(McpServerName::new(server_name)?, tools)?;
    Ok(())
}

/// Registers and starts a server, returning the started registration.
async fn register_and_start(
    service: &TestService,
    ctx: &RequestContext,
    name: &str,
) -> Result<crate::tool_registry::domain::McpServerRegistration> {
    let registered = service.register(ctx, stdio_request(name)?).await?;
    Ok(service.start(ctx, registered.id()).await?.server)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_and_find_by_name(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) -> Result<()> {
    let (_, service) = service_bundle;
    let ctx = test_request_ctx();

    let registered = service
        .register(&ctx, stdio_request("workspace_tools")?)
        .await?;

    let found_server = service
        .find_by_name(&ctx, "workspace_tools")
        .await?
        .expect("server should exist");

    assert_eq!(found_server.id(), registered.id());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn start_unknown_server_returns_not_found(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) {
    let (_, service) = service_bundle;
    let ctx = test_request_ctx();

    let result = service.start(&ctx, McpServerId::new()).await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::NotFound(_))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_tools_requires_running_server(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) -> Result<()> {
    let (_, service) = service_bundle;
    let ctx = test_request_ctx();
    let registered = service
        .register(&ctx, stdio_request("workspace_tools")?)
        .await?;

    let result = service.list_tools(&ctx, registered.id()).await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::Domain(
            ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
        ))
    ));
    Ok(())
}

#[rstest]
#[case::start_and_list(LifecycleScenario::StartAndList)]
#[case::stop_blocks_queries(LifecycleScenario::StopBlocksQueries)]
#[case::refresh_health(LifecycleScenario::RefreshHealth)]
#[case::start_stop_start(LifecycleScenario::StartStopStart)]
#[tokio::test(flavor = "multi_thread")]
async fn lifecycle_scenarios(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
    #[case] scenario: LifecycleScenario,
) -> Result<()> {
    let (host, service) = service_bundle;
    let ctx = test_request_ctx();
    setup_tool_catalog(&host, "workspace_tools", vec![create_read_file_tool()?])?;

    let started = register_and_start(&service, &ctx, "workspace_tools").await?;
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);
    match scenario {
        LifecycleScenario::StartAndList => {
            let tools = service.list_tools(&ctx, started.id()).await?;
            assert_eq!(tools.len(), 1);
            let first = tools.first().expect("tool should exist");
            assert_eq!(first.name(), "read_file");
        }
        LifecycleScenario::StopBlocksQueries => {
            let stopped = service.stop(&ctx, started.id()).await?;
            assert_eq!(stopped.lifecycle_state(), McpServerLifecycleState::Stopped);
            assert!(matches!(
                service.list_tools(&ctx, started.id()).await,
                Err(McpServerLifecycleServiceError::Domain(
                    ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
                ))
            ));
        }
        LifecycleScenario::RefreshHealth => {
            host.set_unhealthy(started.id(), "probe timeout")?;

            let refreshed = service.refresh_health(&ctx, started.id()).await?;

            assert_eq!(
                refreshed.lifecycle_state(),
                McpServerLifecycleState::Running
            );
            let health = refreshed
                .last_health()
                .expect("health snapshot should exist");
            assert_eq!(health.status(), McpServerHealthStatus::Unhealthy);
            assert_eq!(health.message(), Some("probe timeout"));
        }
        LifecycleScenario::StartStopStart => {
            let stopped = service.stop(&ctx, started.id()).await?;
            assert_eq!(stopped.lifecycle_state(), McpServerLifecycleState::Stopped);

            let restarted = service.start(&ctx, started.id()).await?.server;
            assert_eq!(
                restarted.lifecycle_state(),
                McpServerLifecycleState::Running
            );
            assert_eq!(service.list_tools(&ctx, started.id()).await?.len(), 1);
        }
    }
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn start_returns_health_refresh_failed_when_health_probe_fails() -> Result<()> {
    let service = McpServerLifecycleService::new(
        Arc::new(InMemoryMcpServerRegistry::new()),
        Arc::new(HealthProbeFailureHost::default()),
        Arc::new(DefaultClock),
    );
    let ctx = test_request_ctx();

    let registered = service
        .register(&ctx, stdio_request("workspace_tools")?)
        .await?;
    let result = service.start(&ctx, registered.id()).await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::HealthRefreshFailed { .. })
    ));
    Ok(())
}
