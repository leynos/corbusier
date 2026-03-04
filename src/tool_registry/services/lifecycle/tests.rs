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
use eyre::Result;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use std::{collections::HashSet, io::Error, sync::Arc};

type TestService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

#[derive(Debug, Clone, Copy)]
enum LifecycleScenario {
    StartAndList,
    StopBlocksQueries,
    RefreshHealth,
    StartStopStart,
}

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
    name: &str,
) -> Result<crate::tool_registry::domain::McpServerRegistration> {
    let registered = service.register(stdio_request(name)?).await?;
    Ok(service.start(registered.id()).await?)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn register_and_find_by_name(
    service_bundle: (Arc<InMemoryMcpServerHost>, TestService),
) -> Result<()> {
    let (_, service) = service_bundle;

    let registered = service.register(stdio_request("workspace_tools")?).await?;

    let found_server = service
        .find_by_name("workspace_tools")
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
) -> Result<()> {
    let (_, service) = service_bundle;
    let registered = service.register(stdio_request("workspace_tools")?).await?;

    let result = service.list_tools(registered.id()).await;

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
    setup_tool_catalog(&host, "workspace_tools", vec![create_read_file_tool()?])?;

    let started = register_and_start(&service, "workspace_tools").await?;
    assert_eq!(started.lifecycle_state(), McpServerLifecycleState::Running);
    match scenario {
        LifecycleScenario::StartAndList => {
            let tools = service.list_tools(started.id()).await?;
            assert_eq!(tools.len(), 1);
            let first = tools.first().expect("tool should exist");
            assert_eq!(first.name(), "read_file");
        }
        LifecycleScenario::StopBlocksQueries => {
            let stopped = service.stop(started.id()).await?;
            assert_eq!(stopped.lifecycle_state(), McpServerLifecycleState::Stopped);
            assert!(matches!(
                service.list_tools(started.id()).await,
                Err(McpServerLifecycleServiceError::Domain(
                    ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
                ))
            ));
        }
        LifecycleScenario::RefreshHealth => {
            host.set_unhealthy(started.id(), "probe timeout")?;

            let refreshed = service.refresh_health(started.id()).await?;

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
            let stopped = service.stop(started.id()).await?;
            assert_eq!(stopped.lifecycle_state(), McpServerLifecycleState::Stopped);

            let restarted = service.start(started.id()).await?;
            assert_eq!(
                restarted.lifecycle_state(),
                McpServerLifecycleState::Running
            );
            assert_eq!(service.list_tools(started.id()).await?.len(), 1);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct HealthProbeFailureHost {
    started: std::sync::Arc<std::sync::Mutex<HashSet<McpServerId>>>,
}

impl HealthProbeFailureHost {
    /// Helper to acquire the lock on started servers and handle errors.
    fn with_started_lock<F, T>(&self, operation: F) -> McpServerHostResult<T>
    where
        F: FnOnce(&mut HashSet<McpServerId>) -> T,
    {
        let mut started = self
            .started
            .lock()
            .map_err(|err| McpServerHostError::runtime(Error::other(err.to_string())))?;
        Ok(operation(&mut started))
    }
}

#[async_trait]
impl McpServerHost for HealthProbeFailureHost {
    async fn start(
        &self,
        server: &crate::tool_registry::domain::McpServerRegistration,
    ) -> McpServerHostResult<()> {
        self.with_started_lock(|started| {
            started.insert(server.id());
        })
    }

    async fn stop(
        &self,
        server: &crate::tool_registry::domain::McpServerRegistration,
    ) -> McpServerHostResult<()> {
        self.with_started_lock(|started| {
            started.remove(&server.id());
        })
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
        self.with_started_lock(|started| {
            if started.contains(&server.id()) {
                Ok(vec![])
            } else {
                Err(McpServerHostError::NotRunning(server.id()))
            }
        })?
    }
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn start_persists_running_state_when_health_probe_fails() -> Result<()> {
    let service = McpServerLifecycleService::new(
        Arc::new(InMemoryMcpServerRegistry::new()),
        Arc::new(HealthProbeFailureHost::default()),
        Arc::new(DefaultClock),
    );

    let registered = service.register(stdio_request("workspace_tools")?).await?;
    let start_result = service.start(registered.id()).await;

    assert!(matches!(
        start_result,
        Err(McpServerLifecycleServiceError::Host(_))
    ));

    let persisted_server = service
        .find_by_name("workspace_tools")
        .await?
        .expect("server should exist");
    assert_eq!(
        persisted_server.lifecycle_state(),
        McpServerLifecycleState::Running
    );
    assert_eq!(
        persisted_server
            .last_health()
            .expect("health snapshot should exist")
            .status(),
        McpServerHealthStatus::Unknown
    );
    Ok(())
}
