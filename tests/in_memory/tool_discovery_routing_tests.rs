//! In-memory integration tests for tool discovery and call routing.

use std::sync::Arc;

use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
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
        McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
        ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError,
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
fn request_ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}

#[fixture]
fn integration_ctx() -> IntegrationContext {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let clock = Arc::new(DefaultClock);

    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
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

/// Registers a server, starts it, sets up the tool catalog, and discovers
/// tools. Returns the server identifier for further assertions.
async fn register_start_discover(
    request_ctx: &RequestContext,
    ctx: &IntegrationContext,
    server_name: &str,
    tools: Vec<McpToolDefinition>,
) -> Result<corbusier::tool_registry::domain::McpServerId> {
    ctx.host
        .set_tool_catalog(McpServerName::new(server_name)?, tools)?;
    let registered = ctx
        .lifecycle
        .register(request_ctx, stdio_request(server_name)?)
        .await?;
    ctx.lifecycle.start(request_ctx, registered.id()).await?;
    ctx.discovery
        .discover_and_persist_tools(registered.id())
        .await?;
    Ok(registered.id())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn discover_and_call_tool_end_to_end(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let server_id = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "workspace_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello world"}),
    )?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = integration_ctx.discovery.call_tool(&request).await?;
    assert!(result.outcome().is_success());
    assert_eq!(result.server_id(), server_id);
    assert_eq!(result.tool_name(), "read_file");

    let audits = integration_ctx.catalog.audit_records()?;
    assert_eq!(audits.len(), 1);
    assert_eq!(
        audits.first().expect("audit record").tool_name(),
        "read_file"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn two_servers_route_correctly(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let reg1 = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "file_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("file_tools")?,
        "read_file",
        json!({"content": "file contents"}),
    )?;

    let reg2 = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "search_tools",
        vec![search_code_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("search_tools")?,
        "search_code",
        json!({"matches": 3}),
    )?;

    let read_req =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert_eq!(
        integration_ctx
            .discovery
            .call_tool(&read_req)
            .await?
            .server_id(),
        reg1
    );

    let search_req = ToolCallRequest::new("search_code", json!({"query": "hello"}), &DefaultClock);
    assert_eq!(
        integration_ctx
            .discovery
            .call_tool(&search_req)
            .await?
            .server_id(),
        reg2
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn tool_unavailable_after_stop(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let server_id = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "workspace_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx
        .lifecycle
        .stop(&request_ctx, server_id)
        .await?;
    integration_ctx
        .discovery
        .mark_tools_unavailable(server_id)
        .await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert!(matches!(
        integration_ctx.discovery.call_tool(&request).await,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolUnavailable { .. }
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn rediscovery_after_restart(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let server_id = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "workspace_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;

    // Stop and mark unavailable.
    integration_ctx
        .lifecycle
        .stop(&request_ctx, server_id)
        .await?;
    integration_ctx
        .discovery
        .mark_tools_unavailable(server_id)
        .await?;

    // Restart and rediscover.
    integration_ctx
        .lifecycle
        .start(&request_ctx, server_id)
        .await?;
    integration_ctx
        .discovery
        .discover_and_persist_tools(server_id)
        .await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = integration_ctx.discovery.call_tool(&request).await?;
    assert!(result.outcome().is_success());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn audit_trail_accumulates(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    register_start_discover(
        &request_ctx,
        &integration_ctx,
        "workspace_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;

    for _ in 0..3 {
        let request =
            ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
        integration_ctx.discovery.call_tool(&request).await?;
    }
    assert_eq!(integration_ctx.catalog.audit_records()?.len(), 3);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn stderr_captured_for_startup_and_tool_calls(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    // Configure startup stderr on the host before starting.
    let startup_bytes = bytes::Bytes::from("server initialising...");
    integration_ctx.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    integration_ctx.host.set_startup_stderr(
        McpServerName::new("workspace_tools")?,
        startup_bytes.clone(),
    )?;

    // Start via lifecycle -- startup stderr flows through StartHostResult.
    let registered = integration_ctx
        .lifecycle
        .register(&request_ctx, stdio_request("workspace_tools")?)
        .await?;
    let start_result = integration_ctx
        .lifecycle
        .start(&request_ctx, registered.id())
        .await?;
    let captured = start_result
        .startup_stderr
        .expect("startup stderr should be captured");
    assert_eq!(captured, startup_bytes);

    // Persist startup stderr via discovery service.
    let startup_meta = integration_ctx
        .discovery
        .store_startup_stderr(registered.id(), captured)
        .await?;
    assert!(startup_meta.object_path().contains("startup"));

    // Discover tools and configure tool call results.
    integration_ctx
        .discovery
        .discover_and_persist_tools(registered.id())
        .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;
    integration_ctx.host.set_tool_call_stderr(
        McpServerName::new("workspace_tools")?,
        "read_file",
        bytes::Bytes::from("debug: reading file"),
    )?;

    // Call tool and verify audit trail references stderr.
    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    integration_ctx.discovery.call_tool(&request).await?;

    let audits = integration_ctx.catalog.audit_records()?;
    assert_eq!(audits.len(), 1);
    let record = audits.first().expect("audit record");
    assert!(record.stderr_log_path().is_some());
    Ok(())
}
