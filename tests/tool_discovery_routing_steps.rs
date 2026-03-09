//! Behaviour tests for tool discovery and call routing.

use std::sync::Arc;

use corbusier::tool_registry::{
    adapters::{
        AllowAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter,
        memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
    },
    domain::{
        LogRetentionPolicy, McpServerName, McpServerRegistration, McpToolDefinition, McpTransport,
        ToolCallRequest, ToolRegistryDomainError,
    },
    services::{
        McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
        ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError,
    },
};
use eyre::{WrapErr, eyre};
use mockable::DefaultClock;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use serde_json::json;

type TestLifecycleService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

type TestDiscoveryService = ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    AllowAllPolicy,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

#[derive(Default)]
struct ToolDiscoveryWorld {
    host: Arc<InMemoryMcpServerHost>,
    lifecycle: Option<TestLifecycleService>,
    discovery: Option<TestDiscoveryService>,
    catalog: Arc<InMemoryToolCatalog>,
    pending_name: Option<String>,
    pending_command: Option<String>,
    registered_server: Option<McpServerRegistration>,
    last_call_succeeded: Option<bool>,
    last_error: Option<ToolDiscoveryRoutingServiceError>,
}

impl ToolDiscoveryWorld {
    fn new() -> Self {
        let registry = Arc::new(InMemoryMcpServerRegistry::new());
        let host = Arc::new(InMemoryMcpServerHost::new());
        let catalog = Arc::new(InMemoryToolCatalog::new());
        let clock = Arc::new(DefaultClock);

        let lifecycle =
            McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
        let discovery = ToolDiscoveryRoutingService::new(
            ServicePorts {
                catalog: catalog.clone(),
                registry,
                host: host.clone(),
                policy: Arc::new(AllowAllPolicy),
                log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
            },
            LogRetentionPolicy::default(),
            clock,
        );

        Self {
            host,
            lifecycle: Some(lifecycle),
            discovery: Some(discovery),
            catalog,
            pending_name: None,
            pending_command: None,
            registered_server: None,
            last_call_succeeded: None,
            last_error: None,
        }
    }

    fn lifecycle(&self) -> Result<&TestLifecycleService, eyre::Report> {
        self.lifecycle
            .as_ref()
            .ok_or_else(|| eyre!("lifecycle service should exist"))
    }

    fn discovery(&self) -> Result<&TestDiscoveryService, eyre::Report> {
        self.discovery
            .as_ref()
            .ok_or_else(|| eyre!("discovery service should exist"))
    }

    fn pending_name(&self) -> Result<&str, eyre::Report> {
        self.pending_name
            .as_deref()
            .ok_or_else(|| eyre!("pending server name should exist"))
    }
}

#[fixture]
fn world() -> ToolDiscoveryWorld {
    ToolDiscoveryWorld::new()
}

fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

fn request_from_world(
    world: &ToolDiscoveryWorld,
) -> Result<RegisterMcpServerRequest, eyre::Report> {
    let command = world
        .pending_command
        .as_deref()
        .ok_or_else(|| eyre!("pending command should exist"))?;
    Ok(RegisterMcpServerRequest::new(
        world.pending_name()?,
        McpTransport::stdio(command).wrap_err("valid stdio transport expected")?,
    ))
}

// -- Given steps --

#[given(r#"a stdio MCP server named "{name}" with command "{command}""#)]
fn stdio_server_definition(world: &mut ToolDiscoveryWorld, name: String, command: String) {
    world.pending_name = Some(name);
    world.pending_command = Some(command);
}

#[given(r#"tool "{tool_name}" is available on that server"#)]
fn tool_available_on_server(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let tool = McpToolDefinition::new(
        &tool_name,
        format!("Tool {tool_name}"),
        json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
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

#[given(r#"calling tool "{tool_name}" on that server returns '{result}'"#)]
fn tool_call_result_configured(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
    result: String,
) -> Result<(), eyre::Report> {
    let value: serde_json::Value =
        serde_json::from_str(&result).wrap_err("result should be valid JSON")?;
    world
        .host
        .set_tool_call_result(
            McpServerName::new(world.pending_name()?).wrap_err("valid pending name expected")?,
            &tool_name,
            value,
        )
        .wrap_err("tool call result setup should succeed")?;
    Ok(())
}

#[given(r#"calling tool "{tool_name}" on that server produces stderr "{stderr}""#)]
fn tool_call_stderr_configured(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
    stderr: String,
) -> Result<(), eyre::Report> {
    world
        .host
        .set_tool_call_stderr(
            McpServerName::new(world.pending_name()?).wrap_err("valid pending name expected")?,
            &tool_name,
            bytes::Bytes::from(stderr),
        )
        .wrap_err("tool call stderr setup should succeed")?;
    Ok(())
}

// -- When steps --

#[when("the server is registered and started")]
fn register_and_start_server(world: &mut ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    let registered = run_async(world.lifecycle()?.register(request_from_world(world)?))
        .wrap_err("registration should succeed")?;
    let start_result =
        run_async(world.lifecycle()?.start(registered.id())).wrap_err("start should succeed")?;
    world.registered_server = Some(start_result.server);
    Ok(())
}

#[when("tools are discovered")]
fn discover_tools(world: &mut ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("server should be registered"))?;
    run_async(world.discovery()?.discover_and_persist_tools(server.id()))
        .wrap_err("tool discovery should succeed")?;
    Ok(())
}

#[when("the server is stopped")]
fn stop_server(world: &mut ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("server should be registered"))?;
    let stopped =
        run_async(world.lifecycle()?.stop(server.id())).wrap_err("stop should succeed")?;
    world.registered_server = Some(stopped);
    Ok(())
}

#[when("tools are marked unavailable")]
fn mark_tools_unavailable(world: &mut ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("server should be registered"))?;
    run_async(world.discovery()?.mark_tools_unavailable(server.id()))
        .wrap_err("mark unavailable should succeed")?;
    Ok(())
}

#[when(r#"tool "{tool_name}" is called with parameters '{params}'"#)]
fn call_tool_with_params(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
    params: String,
) -> Result<(), eyre::Report> {
    let parameters: serde_json::Value =
        serde_json::from_str(&params).wrap_err("parameters should be valid JSON")?;
    let request = ToolCallRequest::new(&tool_name, parameters, &DefaultClock);
    match run_async(world.discovery()?.call_tool(&request)) {
        Ok(_) => {
            world.last_call_succeeded = Some(true);
        }
        Err(err) => {
            world.last_call_succeeded = Some(false);
            world.last_error = Some(err);
        }
    }
    Ok(())
}

// -- Then steps --

#[then(r"the tool catalog contains {count:usize} entry")]
fn catalog_contains_count(
    world: &mut ToolDiscoveryWorld,
    count: usize,
) -> Result<(), eyre::Report> {
    let entries =
        run_async(world.discovery()?.list_catalog()).wrap_err("catalog listing should succeed")?;
    if entries.len() != count {
        return Err(eyre!(
            "expected {count} catalog entries, got {}",
            entries.len()
        ));
    }
    Ok(())
}

#[then(r#"tool "{tool_name}" is marked as available"#)]
fn tool_is_available(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let entries =
        run_async(world.discovery()?.list_catalog()).wrap_err("catalog listing should succeed")?;
    let entry = entries
        .iter()
        .find(|e| e.tool().name() == tool_name)
        .ok_or_else(|| eyre!("tool '{tool_name}' not found in catalog"))?;
    if !entry.available() {
        return Err(eyre!("tool '{tool_name}' should be available but is not"));
    }
    Ok(())
}

#[then("the tool call succeeds")]
fn tool_call_succeeds(world: &ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    match world.last_call_succeeded {
        Some(true) => Ok(()),
        Some(false) => Err(eyre!(
            "tool call should have succeeded but failed: {:?}",
            world.last_error
        )),
        None => Err(eyre!("no tool call was made")),
    }
}

#[then(r#"the audit log contains {count:usize} entry for tool "{tool_name}""#)]
fn audit_log_contains_entry(
    world: &ToolDiscoveryWorld,
    count: usize,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let records = world
        .catalog
        .audit_records()
        .map_err(|err| eyre!("audit records retrieval failed: {err}"))?;
    let matching: Vec<_> = records
        .iter()
        .filter(|r| r.tool_name() == tool_name)
        .collect();
    if matching.len() != count {
        return Err(eyre!(
            "expected {count} audit entries for '{tool_name}', got {}",
            matching.len()
        ));
    }
    Ok(())
}

#[then(r#"calling tool "{tool_name}" is rejected as unavailable"#)]
fn tool_call_rejected_unavailable(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let request = ToolCallRequest::new(&tool_name, json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = run_async(world.discovery()?.call_tool(&request));
    if !matches!(
        result,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolUnavailable { .. }
        ))
    ) {
        return Err(eyre!("expected ToolUnavailable error, got {result:?}"));
    }
    Ok(())
}

#[then(r#"calling tool "{tool_name}" is rejected as not found"#)]
fn tool_call_rejected_not_found(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let request = ToolCallRequest::new(&tool_name, json!({}), &DefaultClock);
    let result = run_async(world.discovery()?.call_tool(&request));
    if !matches!(
        result,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolNotFound(_)
        ))
    ) {
        return Err(eyre!("expected ToolNotFound error, got {result:?}"));
    }
    Ok(())
}

#[then(r#"the audit log entry for tool "{tool_name}" has a stderr log path"#)]
fn audit_entry_has_stderr_log_path(
    world: &ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let records = world
        .catalog
        .audit_records()
        .map_err(|err| eyre!("audit records retrieval failed: {err}"))?;
    let entry = records
        .iter()
        .find(|r| r.tool_name() == tool_name)
        .ok_or_else(|| eyre!("no audit entry for tool '{tool_name}'"))?;
    if entry.stderr_log_path().is_none() {
        return Err(eyre!(
            "audit entry for '{tool_name}' should have a stderr log path"
        ));
    }
    Ok(())
}

// -- Scenario bindings --

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Discover tools from a running MCP server"
)]
#[tokio::test(flavor = "multi_thread")]
async fn discover_tools_from_running_server(world: ToolDiscoveryWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Route a tool call to the correct server"
)]
#[tokio::test(flavor = "multi_thread")]
async fn route_tool_call_to_correct_server(world: ToolDiscoveryWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Tool becomes unavailable when server stops"
)]
#[tokio::test(flavor = "multi_thread")]
async fn tool_unavailable_when_server_stops(world: ToolDiscoveryWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Unknown tool call is rejected"
)]
#[tokio::test(flavor = "multi_thread")]
async fn unknown_tool_call_rejected(world: ToolDiscoveryWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tool_discovery_routing.feature",
    name = "Tool call stderr is captured in the log store"
)]
#[tokio::test(flavor = "multi_thread")]
async fn tool_call_stderr_captured(world: ToolDiscoveryWorld) {
    let _ = world;
}
