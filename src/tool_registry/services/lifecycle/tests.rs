//! Unit tests for MCP server lifecycle orchestration.

use super::{McpServerLifecycleService, McpServerLifecycleServiceError, RegisterMcpServerRequest};
use crate::tool_registry::{
    adapters::{InMemoryMcpServerHost, memory::InMemoryMcpServerRegistry},
    domain::{
        McpServerHealthSnapshot, McpServerHealthStatus, McpServerId, McpServerLifecycleState,
        McpServerName, McpToolDefinition, McpTransport, ToolRegistryDomainError,
    },
    ports::{McpServerHost, McpServerHostError, McpServerHostResult},
};
use async_trait::async_trait;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use std::{io::Error, sync::Arc};

type TestService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

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
fn create_read_file_tool() -> McpToolDefinition {
    McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "properties": {"path": {"type": "string"}}}),
    )
    .expect("tool definition should be valid")
}

/// Sets up a tool catalog for the given server on the host.
fn setup_tool_catalog(
    host: &Arc<InMemoryMcpServerHost>,
    server_name: &str,
    tools: Vec<McpToolDefinition>,
) {
    host.set_tool_catalog(McpServerName::new(server_name).expect("valid name"), tools)
        .expect("catalog update should succeed");
}

/// Registers and starts a server, returning the started registration.
async fn register_and_start(
    service: &TestService,
    name: &str,
) -> crate::tool_registry::domain::McpServerRegistration {
    let registered = service
        .register(stdio_request(name).expect("valid stdio transport"))
        .await
        .expect("registration should succeed");
    service
        .start(registered.id())
        .await
        .expect("start should succeed")
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_and_find_by_name(service_bundle: (Arc<InMemoryMcpServerHost>, TestService)) {
    let (_, service) = service_bundle;

    let registered = service
        .register(stdio_request("workspace_tools").expect("valid stdio transport"))
        .await
        .expect("registration should succeed");

    let found = service
        .find_by_name("workspace_tools")
        .await
        .expect("lookup should succeed")
        .expect("server should exist");

    assert_eq!(found.id(), registered.id());
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn start_unknown_server_returns_not_found(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) {
    let (_, service) = service_bundle;

    let result = service.start(McpServerId::new()).await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::NotFound(_))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_tools_requires_running_server(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) {
    let (_, service) = service_bundle;
    let registered = service
        .register(stdio_request("workspace_tools").expect("valid stdio transport"))
        .await
        .expect("registration should succeed");

    let result = service.list_tools(registered.id()).await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::Domain(
            ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn start_and_list_tools(service_bundle: (Arc<InMemoryMcpServerHost>, TestService)) {
    let (host, service) = service_bundle;
    setup_tool_catalog(&host, "workspace_tools", vec![create_read_file_tool()]);

    let started = register_and_start(&service, "workspace_tools").await;
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);

    let tools = service
        .list_tools(started.id())
        .await
        .expect("tool listing should succeed");

    assert_eq!(tools.len(), 1);
    let first = tools.first().expect("tool should exist");
    assert_eq!(first.name(), "read_file");
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn stop_transitions_to_stopped_and_blocks_tool_queries(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) {
    let (host, service) = service_bundle;
    setup_tool_catalog(&host, "workspace_tools", vec![create_read_file_tool()]);

    let started = register_and_start(&service, "workspace_tools").await;
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);

    let stopped = service
        .stop(started.id())
        .await
        .expect("stop should succeed");
    assert_eq!(stopped.lifecycle_state(), McpServerLifecycleState::Stopped);
    assert!(matches!(
        service.list_tools(started.id()).await,
        Err(McpServerLifecycleServiceError::Domain(
            ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn refresh_health_updates_snapshot_without_state_change(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) {
    let (host, service) = service_bundle;
    setup_tool_catalog(&host, "workspace_tools", vec![create_read_file_tool()]);

    let started = register_and_start(&service, "workspace_tools").await;
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);

    host.set_unhealthy(started.id(), "probe timeout")
        .expect("marking unhealthy should succeed");

    let refreshed = service
        .refresh_health(started.id())
        .await
        .expect("health refresh should succeed");

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

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn start_stop_start_restart_cycle_returns_to_running(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) {
    let (host, service) = service_bundle;
    setup_tool_catalog(&host, "workspace_tools", vec![create_read_file_tool()]);

    let started = register_and_start(&service, "workspace_tools").await;
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);

    let stopped = service
        .stop(started.id())
        .await
        .expect("stop should succeed");
    assert_eq!(stopped.lifecycle_state(), McpServerLifecycleState::Stopped);

    let restarted = service
        .start(started.id())
        .await
        .expect("restart should succeed");
    assert_eq!(
        restarted.lifecycle_state(),
        McpServerLifecycleState::Running
    );
    assert_eq!(
        service
            .list_tools(started.id())
            .await
            .expect("tool listing should succeed after restart")
            .len(),
        1
    );
}

#[derive(Debug, Clone, Default)]
struct HealthProbeFailureHost {
    started: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<McpServerId>>>,
}

#[async_trait]
impl McpServerHost for HealthProbeFailureHost {
    async fn start(
        &self,
        server: &crate::tool_registry::domain::McpServerRegistration,
    ) -> McpServerHostResult<()> {
        let mut started = self
            .started
            .lock()
            .map_err(|err| McpServerHostError::runtime(Error::other(err.to_string())))?;
        started.insert(server.id());
        Ok(())
    }

    async fn stop(
        &self,
        server: &crate::tool_registry::domain::McpServerRegistration,
    ) -> McpServerHostResult<()> {
        let mut started = self
            .started
            .lock()
            .map_err(|err| McpServerHostError::runtime(Error::other(err.to_string())))?;
        started.remove(&server.id());
        Ok(())
    }

    async fn health(
        &self,
        _server: &crate::tool_registry::domain::McpServerRegistration,
    ) -> McpServerHostResult<McpServerHealthSnapshot> {
        Err(McpServerHostError::runtime(Error::other(
            "health probe unavailable",
        )))
    }

    async fn list_tools(
        &self,
        server: &crate::tool_registry::domain::McpServerRegistration,
    ) -> McpServerHostResult<Vec<McpToolDefinition>> {
        let started = self
            .started
            .lock()
            .map_err(|err| McpServerHostError::runtime(Error::other(err.to_string())))?;
        if started.contains(&server.id()) {
            return Ok(vec![]);
        }
        Err(McpServerHostError::NotRunning(server.id()))
    }
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn start_persists_running_state_when_health_probe_fails() {
    let service = McpServerLifecycleService::new(
        Arc::new(InMemoryMcpServerRegistry::new()),
        Arc::new(HealthProbeFailureHost::default()),
        Arc::new(DefaultClock),
    );

    let registered = service
        .register(stdio_request("workspace_tools").expect("valid stdio transport"))
        .await
        .expect("registration should succeed");
    let start_result = service.start(registered.id()).await;

    assert!(matches!(
        start_result,
        Err(McpServerLifecycleServiceError::Host(_))
    ));

    let persisted = service
        .find_by_name("workspace_tools")
        .await
        .expect("lookup should succeed")
        .expect("server should exist");
    assert_eq!(
        persisted.lifecycle_state(),
        McpServerLifecycleState::Running
    );
    assert_eq!(
        persisted
            .last_health()
            .expect("health snapshot should exist")
            .status(),
        McpServerHealthStatus::Unknown
    );
}
