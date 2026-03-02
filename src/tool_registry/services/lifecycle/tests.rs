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
use rstest::rstest;
use serde_json::json;
use std::{io::Error, sync::Arc};

type TestService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

fn build_service() -> (Arc<InMemoryMcpServerHost>, TestService) {
    let host = Arc::new(InMemoryMcpServerHost::new());
    let service = McpServerLifecycleService::new(
        Arc::new(InMemoryMcpServerRegistry::new()),
        host.clone(),
        Arc::new(DefaultClock),
    );
    (host, service)
}

fn stdio_request(name: &str) -> RegisterMcpServerRequest {
    RegisterMcpServerRequest::new(
        name,
        McpTransport::stdio("mcp-server").expect("valid stdio transport"),
    )
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_and_find_by_name() {
    let (_, service) = build_service();

    let registered = service
        .register(stdio_request("workspace_tools"))
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
async fn start_unknown_server_returns_not_found() {
    let (_, service) = build_service();

    let result = service.start(McpServerId::new()).await;

    assert!(matches!(
        result,
        Err(McpServerLifecycleServiceError::NotFound(_))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn list_tools_requires_running_server() {
    let (_, service) = build_service();
    let registered = service
        .register(stdio_request("workspace_tools"))
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
async fn start_and_list_tools() {
    let (host, service) = build_service();

    let tool = McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "properties": {"path": {"type": "string"}}}),
    )
    .expect("tool definition should be valid");

    host.set_tool_catalog(
        McpServerName::new("workspace_tools").expect("valid name"),
        vec![tool],
    )
    .expect("catalog update should succeed");

    let registered = service
        .register(stdio_request("workspace_tools"))
        .await
        .expect("registration should succeed");
    service
        .start(registered.id())
        .await
        .expect("start should succeed");

    let tools = service
        .list_tools(registered.id())
        .await
        .expect("tool listing should succeed");

    assert_eq!(tools.len(), 1);
    let first = tools.first().expect("tool should exist");
    assert_eq!(first.name(), "read_file");
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn stop_transitions_to_stopped_and_blocks_tool_queries() {
    let (host, service) = build_service();
    let tool = McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "properties": {"path": {"type": "string"}}}),
    )
    .expect("tool definition should be valid");

    host.set_tool_catalog(
        McpServerName::new("workspace_tools").expect("valid name"),
        vec![tool],
    )
    .expect("catalog update should succeed");

    let registered = service
        .register(stdio_request("workspace_tools"))
        .await
        .expect("registration should succeed");
    let started = service
        .start(registered.id())
        .await
        .expect("start should succeed");
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);

    let stopped = service
        .stop(registered.id())
        .await
        .expect("stop should succeed");

    assert_eq!(stopped.lifecycle_state(), McpServerLifecycleState::Stopped);
    assert!(matches!(
        service.list_tools(registered.id()).await,
        Err(McpServerLifecycleServiceError::Domain(
            ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
        ))
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn refresh_health_updates_snapshot_without_changing_state() {
    let (host, service) = build_service();
    let registered = service
        .register(stdio_request("workspace_tools"))
        .await
        .expect("registration should succeed");
    let started = service
        .start(registered.id())
        .await
        .expect("start should succeed");
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);

    host.set_unhealthy(registered.id(), "probe timeout")
        .expect("marking unhealthy should succeed");

    let refreshed = service
        .refresh_health(registered.id())
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
async fn start_stop_start_restart_cycle_returns_to_running() {
    let (host, service) = build_service();
    let tool = McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "properties": {"path": {"type": "string"}}}),
    )
    .expect("tool definition should be valid");

    host.set_tool_catalog(
        McpServerName::new("workspace_tools").expect("valid name"),
        vec![tool],
    )
    .expect("catalog update should succeed");

    let registered = service
        .register(stdio_request("workspace_tools"))
        .await
        .expect("registration should succeed");
    let started = service
        .start(registered.id())
        .await
        .expect("first start should succeed");
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);

    let stopped = service
        .stop(registered.id())
        .await
        .expect("stop should succeed");
    assert_eq!(stopped.lifecycle_state(), McpServerLifecycleState::Stopped);

    let restarted = service
        .start(registered.id())
        .await
        .expect("restart should succeed");
    assert_eq!(
        restarted.lifecycle_state(),
        McpServerLifecycleState::Running
    );
    assert_eq!(
        service
            .list_tools(registered.id())
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
        .register(stdio_request("workspace_tools"))
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
