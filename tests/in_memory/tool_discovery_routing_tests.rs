//! In-memory integration tests for tool discovery and call routing.

use std::sync::Arc;

use corbusier::tool_registry::{
    adapters::{
        AllowAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter,
        memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
    },
    domain::{
        LogRetentionPolicy, McpServerName, McpToolDefinition, McpTransport, ToolCallRequest,
        ToolRegistryDomainError,
    },
    services::{
        McpServerLifecycleService, RegisterMcpServerRequest, ToolDiscoveryRoutingService,
        ToolDiscoveryRoutingServiceError,
    },
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
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

struct IntegrationContext {
    host: Arc<InMemoryMcpServerHost>,
    lifecycle: TestLifecycleService,
    discovery: TestDiscoveryService,
    catalog: Arc<InMemoryToolCatalog>,
}

#[fixture]
fn ctx() -> IntegrationContext {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let clock = Arc::new(DefaultClock);

    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let discovery = ToolDiscoveryRoutingService::new(
        catalog.clone(),
        registry,
        host.clone(),
        Arc::new(AllowAllPolicy),
        Arc::new(ObjectStoreLogAdapter::in_memory()),
        LogRetentionPolicy::default(),
        clock,
    );

    IntegrationContext {
        host,
        lifecycle,
        discovery,
        catalog,
    }
}

fn stdio_request(name: &str) -> Result<RegisterMcpServerRequest, ToolRegistryDomainError> {
    let transport = McpTransport::stdio("mcp-server")?;
    Ok(RegisterMcpServerRequest::new(name, transport))
}

fn read_file_tool() -> Result<McpToolDefinition> {
    Ok(McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    )?)
}

fn search_code_tool() -> Result<McpToolDefinition> {
    Ok(McpToolDefinition::new(
        "search_code",
        "Searches the workspace source tree",
        json!({"type": "object", "required": ["query"], "properties": {"query": {"type": "string"}}}),
    )?)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::indexing_slicing,
    reason = "test asserts on elements after verifying counts"
)]
async fn discover_and_call_tool_end_to_end(ctx: IntegrationContext) -> Result<()> {
    ctx.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello world"}),
    )?;

    let registered = ctx
        .lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    ctx.lifecycle.start(registered.id()).await?;

    let entries = ctx
        .discovery
        .discover_and_persist_tools(registered.id())
        .await?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].tool().name(), "read_file");

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = ctx.discovery.call_tool(&request).await?;

    assert!(result.outcome().is_success());
    assert_eq!(result.server_id(), registered.id());
    assert_eq!(result.tool_name(), "read_file");

    let audit_records = ctx.catalog.audit_records()?;
    assert_eq!(audit_records.len(), 1);
    assert_eq!(audit_records[0].tool_name(), "read_file");

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn two_servers_route_correctly(ctx: IntegrationContext) -> Result<()> {
    // Set up server 1 with read_file.
    ctx.host
        .set_tool_catalog(McpServerName::new("file_tools")?, vec![read_file_tool()?])?;
    ctx.host.set_tool_call_result(
        McpServerName::new("file_tools")?,
        "read_file",
        json!({"content": "file contents"}),
    )?;

    // Set up server 2 with search_code.
    ctx.host.set_tool_catalog(
        McpServerName::new("search_tools")?,
        vec![search_code_tool()?],
    )?;
    ctx.host.set_tool_call_result(
        McpServerName::new("search_tools")?,
        "search_code",
        json!({"matches": 3}),
    )?;

    let reg1 = ctx.lifecycle.register(stdio_request("file_tools")?).await?;
    ctx.lifecycle.start(reg1.id()).await?;
    ctx.discovery.discover_and_persist_tools(reg1.id()).await?;

    let reg2 = ctx
        .lifecycle
        .register(stdio_request("search_tools")?)
        .await?;
    ctx.lifecycle.start(reg2.id()).await?;
    ctx.discovery.discover_and_persist_tools(reg2.id()).await?;

    // Call read_file -- should route to file_tools.
    let read_req =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let read_result = ctx.discovery.call_tool(&read_req).await?;
    assert_eq!(read_result.server_id(), reg1.id());

    // Call search_code -- should route to search_tools.
    let search_req = ToolCallRequest::new("search_code", json!({"query": "hello"}), &DefaultClock);
    let search_result = ctx.discovery.call_tool(&search_req).await?;
    assert_eq!(search_result.server_id(), reg2.id());

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn tool_unavailable_after_stop(ctx: IntegrationContext) -> Result<()> {
    ctx.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;

    let registered = ctx
        .lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    ctx.lifecycle.start(registered.id()).await?;
    ctx.discovery
        .discover_and_persist_tools(registered.id())
        .await?;

    ctx.lifecycle.stop(registered.id()).await?;
    ctx.discovery
        .mark_tools_unavailable(registered.id())
        .await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = ctx.discovery.call_tool(&request).await;

    assert!(matches!(
        result,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolUnavailable { .. }
        ))
    ));

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn rediscovery_after_restart(ctx: IntegrationContext) -> Result<()> {
    ctx.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;

    let registered = ctx
        .lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    ctx.lifecycle.start(registered.id()).await?;
    ctx.discovery
        .discover_and_persist_tools(registered.id())
        .await?;

    // Stop and mark unavailable.
    ctx.lifecycle.stop(registered.id()).await?;
    ctx.discovery
        .mark_tools_unavailable(registered.id())
        .await?;

    // Restart and rediscover.
    ctx.lifecycle.start(registered.id()).await?;
    ctx.discovery
        .discover_and_persist_tools(registered.id())
        .await?;

    // Call should succeed again.
    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = ctx.discovery.call_tool(&request).await?;
    assert!(result.outcome().is_success());

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn audit_trail_accumulates(ctx: IntegrationContext) -> Result<()> {
    ctx.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;

    let registered = ctx
        .lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    ctx.lifecycle.start(registered.id()).await?;
    ctx.discovery
        .discover_and_persist_tools(registered.id())
        .await?;

    for _ in 0..3 {
        let request =
            ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
        ctx.discovery.call_tool(&request).await?;
    }

    let audit_records = ctx.catalog.audit_records()?;
    assert_eq!(audit_records.len(), 3);

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::indexing_slicing,
    reason = "test asserts on elements after verifying counts"
)]
async fn stderr_captured_for_startup_and_tool_calls(ctx: IntegrationContext) -> Result<()> {
    ctx.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;
    ctx.host.set_tool_call_stderr(
        McpServerName::new("workspace_tools")?,
        "read_file",
        bytes::Bytes::from("debug: reading file"),
    )?;

    let registered = ctx
        .lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    ctx.lifecycle.start(registered.id()).await?;

    // Store startup stderr.
    let startup_meta = ctx
        .discovery
        .store_startup_stderr(
            registered.id(),
            bytes::Bytes::from("server initialising..."),
        )
        .await?;
    assert!(startup_meta.object_path().contains("startup"));

    // Discover and call tool.
    ctx.discovery
        .discover_and_persist_tools(registered.id())
        .await?;
    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    ctx.discovery.call_tool(&request).await?;

    // Verify audit trail references stderr.
    let audit_records = ctx.catalog.audit_records()?;
    assert_eq!(audit_records.len(), 1);
    assert!(audit_records[0].stderr_log_path().is_some());

    Ok(())
}
