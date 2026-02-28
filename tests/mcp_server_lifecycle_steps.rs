//! Behaviour tests for MCP server lifecycle and tool discovery.

use std::sync::Arc;

use corbusier::tool_registry::{
    adapters::{InMemoryMcpServerHost, memory::InMemoryMcpServerRegistry},
    domain::{McpServerName, McpServerRegistration, McpToolDefinition, McpTransport},
    ports::McpServerRegistryError,
    services::{
        McpServerLifecycleService, McpServerLifecycleServiceError, RegisterMcpServerRequest,
    },
};
use eyre::{WrapErr, eyre};
use mockable::DefaultClock;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use serde_json::json;

type TestService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

#[derive(Default)]
struct McpLifecycleWorld {
    host: Arc<InMemoryMcpServerHost>,
    service: Option<TestService>,
    pending_name: Option<String>,
    pending_command: Option<String>,
    registered_server: Option<McpServerRegistration>,
    last_servers: Option<Vec<McpServerRegistration>>,
    last_tools_count: Option<usize>,
    last_error: Option<McpServerLifecycleServiceError>,
}

impl McpLifecycleWorld {
    fn new() -> Self {
        let host = Arc::new(InMemoryMcpServerHost::new());
        let service = McpServerLifecycleService::new(
            Arc::new(InMemoryMcpServerRegistry::new()),
            host.clone(),
            Arc::new(DefaultClock),
        );

        Self {
            host,
            service: Some(service),
            pending_name: None,
            pending_command: None,
            registered_server: None,
            last_servers: None,
            last_tools_count: None,
            last_error: None,
        }
    }

    fn service(&self) -> Result<&TestService, eyre::Report> {
        self.service
            .as_ref()
            .ok_or_else(|| eyre!("test service should exist"))
    }

    fn pending_name(&self) -> Result<&str, eyre::Report> {
        self.pending_name
            .as_deref()
            .ok_or_else(|| eyre!("pending server name should exist"))
    }
}

#[fixture]
fn world() -> McpLifecycleWorld {
    McpLifecycleWorld::new()
}

fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

fn request_from_world(world: &McpLifecycleWorld) -> Result<RegisterMcpServerRequest, eyre::Report> {
    let command = world
        .pending_command
        .as_deref()
        .ok_or_else(|| eyre!("pending command should exist"))?;
    Ok(RegisterMcpServerRequest::new(
        world.pending_name()?,
        McpTransport::stdio(command).wrap_err("valid stdio transport expected")?,
    ))
}

#[given(r#"a stdio MCP server named "{name}" with command "{command}""#)]
fn stdio_server_definition(world: &mut McpLifecycleWorld, name: String, command: String) {
    world.pending_name = Some(name);
    world.pending_command = Some(command);
}

#[given(r#"tool "{tool_name}" is available on that server"#)]
fn tool_available_on_server(
    world: &mut McpLifecycleWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let tool = McpToolDefinition::new(
        &tool_name,
        format!("Tool {tool_name}"),
        json!({"type": "object"}),
    )
    .wrap_err("tool definition should be valid")?;

    world
        .host
        .set_tool_catalog(
            McpServerName::new(world.pending_name()?).wrap_err("valid pending name expected")?,
            vec![tool],
        )
        .wrap_err("catalog setup should succeed")?;

    Ok(())
}

#[when("the server is registered")]
fn register_server(world: &mut McpLifecycleWorld) -> Result<(), eyre::Report> {
    let registered = run_async(world.service()?.register(request_from_world(world)?))
        .wrap_err("registration should succeed")?;
    world.registered_server = Some(registered);
    Ok(())
}

#[when("the server is registered twice")]
fn register_server_twice(world: &mut McpLifecycleWorld) -> Result<(), eyre::Report> {
    run_async(world.service()?.register(request_from_world(world)?))
        .wrap_err("first registration should succeed")?;
    let second_result = run_async(world.service()?.register(request_from_world(world)?));
    world.last_error = second_result.err();
    Ok(())
}

#[when("the server is started")]
fn start_server(world: &mut McpLifecycleWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("server should be registered"))?;

    let started =
        run_async(world.service()?.start(server.id())).wrap_err("start should succeed")?;
    world.registered_server = Some(started);
    Ok(())
}

#[when("the server is stopped")]
fn stop_server(world: &mut McpLifecycleWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("server should be registered"))?;

    let stopped = run_async(world.service()?.stop(server.id())).wrap_err("stop should succeed")?;
    world.registered_server = Some(stopped);
    Ok(())
}

#[then(r"listing all servers returns {count:usize} entries")]
fn list_servers_returns_count(
    world: &mut McpLifecycleWorld,
    count: usize,
) -> Result<(), eyre::Report> {
    let servers = run_async(world.service()?.list_all()).wrap_err("listing should succeed")?;
    world.last_servers = Some(servers.clone());
    if servers.len() != count {
        return Err(eyre!("expected {count} servers, got {}", servers.len()));
    }
    Ok(())
}

#[then(r#"the server lifecycle state is "{state}""#)]
fn server_lifecycle_state(world: &McpLifecycleWorld, state: String) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("registered server should exist"))?;
    if server.lifecycle_state().as_str() != state {
        return Err(eyre!(
            "expected lifecycle state '{state}', got '{}'",
            server.lifecycle_state().as_str()
        ));
    }
    Ok(())
}

#[then(r"querying tools returns {count:usize} entries")]
fn query_tools_returns_count(
    world: &mut McpLifecycleWorld,
    count: usize,
) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("registered server should exist"))?;

    let tools = run_async(world.service()?.list_tools(server.id()))
        .wrap_err("tool query should succeed")?;
    world.last_tools_count = Some(tools.len());
    if tools.len() != count {
        return Err(eyre!("expected {count} tools, got {}", tools.len()));
    }
    Ok(())
}

#[then("registration fails with a duplicate server name error")]
fn registration_fails_with_duplicate_name(world: &McpLifecycleWorld) -> Result<(), eyre::Report> {
    let error = world
        .last_error
        .as_ref()
        .ok_or_else(|| eyre!("expected registration error"))?;

    if !matches!(
        error,
        McpServerLifecycleServiceError::Repository(McpServerRegistryError::DuplicateServerName(_))
    ) {
        return Err(eyre!("expected duplicate name error, got {error:?}"));
    }
    Ok(())
}

#[then("querying tools is rejected because the server is not running")]
fn query_tools_rejected_when_not_running(world: &McpLifecycleWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("registered server should exist"))?;

    let result = run_async(world.service()?.list_tools(server.id()));
    if !matches!(
        result,
        Err(McpServerLifecycleServiceError::Domain(
            corbusier::tool_registry::domain::ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
        ))
    ) {
        return Err(eyre!("expected not-running tool query error"));
    }

    Ok(())
}

#[scenario(
    path = "tests/features/mcp_server_lifecycle.feature",
    name = "Register, start, and query tools from an MCP server"
)]
#[tokio::test(flavor = "multi_thread")]
async fn register_start_and_query(world: McpLifecycleWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/mcp_server_lifecycle.feature",
    name = "Reject duplicate MCP server registration"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_duplicate_registration(world: McpLifecycleWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/mcp_server_lifecycle.feature",
    name = "Stopped server rejects tool queries"
)]
#[tokio::test(flavor = "multi_thread")]
async fn stopped_server_rejects_tool_queries(world: McpLifecycleWorld) {
    let _ = world;
}
