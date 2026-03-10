//! Shared test infrastructure for tool discovery and routing tests.

use crate::{
    context::{CorrelationId, RequestContext, SessionId, TenantId, UserId},
    tool_registry::{
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
            ToolDiscoveryRoutingServiceResult,
        },
    },
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::fixture;
use serde_json::json;
use std::sync::Arc;

pub type TestLifecycleService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;
pub type TestDiscoveryService = ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    AllowAllPolicy,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

pub fn test_request_ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}

pub struct TestBundle {
    pub host: Arc<InMemoryMcpServerHost>,
    pub lifecycle: TestLifecycleService,
    pub discovery: TestDiscoveryService,
    pub catalog: Arc<InMemoryToolCatalog>,
}

#[fixture]
pub fn bundle() -> TestBundle {
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
    TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
    }
}

pub fn stdio_request(name: &str) -> Result<RegisterMcpServerRequest, ToolRegistryDomainError> {
    Ok(RegisterMcpServerRequest::new(
        name,
        McpTransport::stdio("mcp-server")?,
    ))
}

pub fn read_file_tool() -> Result<McpToolDefinition> {
    Ok(McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    )?)
}

pub async fn register_start_discover<Pol: crate::tool_registry::ports::ToolPolicyEnforcer>(
    host: &InMemoryMcpServerHost,
    lifecycle: &TestLifecycleService,
    discovery: &ToolDiscoveryRoutingService<
        InMemoryToolCatalog,
        InMemoryMcpServerRegistry,
        InMemoryMcpServerHost,
        Pol,
        ObjectStoreLogAdapter,
        DefaultClock,
    >,
    ctx: &RequestContext,
) -> Result<crate::tool_registry::domain::McpServerId> {
    host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    let registered = lifecycle
        .register(ctx, stdio_request("workspace_tools")?)
        .await?;
    lifecycle.start(ctx, registered.id()).await?;
    discovery
        .discover_and_persist_tools(ctx, registered.id())
        .await?;
    Ok(registered.id())
}

/// Registers, starts (with startup stderr), and discovers tools.
/// Returns the server identifier and the captured startup stderr
/// bytes.
#[expect(
    clippy::too_many_arguments,
    reason = "test helper that threads through all service components plus context"
)]
pub async fn register_start_with_stderr<Pol: crate::tool_registry::ports::ToolPolicyEnforcer>(
    host: &InMemoryMcpServerHost,
    lifecycle: &TestLifecycleService,
    discovery: &ToolDiscoveryRoutingService<
        InMemoryToolCatalog,
        InMemoryMcpServerRegistry,
        InMemoryMcpServerHost,
        Pol,
        ObjectStoreLogAdapter,
        DefaultClock,
    >,
    ctx: &RequestContext,
    startup_stderr: bytes::Bytes,
) -> Result<(
    crate::tool_registry::domain::McpServerId,
    Option<bytes::Bytes>,
)> {
    host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    host.set_startup_stderr(McpServerName::new("workspace_tools")?, startup_stderr)?;
    let registered = lifecycle
        .register(ctx, stdio_request("workspace_tools")?)
        .await?;
    let start_result = lifecycle.start(ctx, registered.id()).await?;
    discovery
        .discover_and_persist_tools(ctx, registered.id())
        .await?;
    Ok((registered.id(), start_result.startup_stderr))
}

pub async fn call_read_file(
    ctx: &RequestContext,
    discovery: &TestDiscoveryService,
    params: serde_json::Value,
) -> ToolDiscoveryRoutingServiceResult<crate::tool_registry::domain::ToolCallResult> {
    discovery
        .call_tool(
            ctx,
            &ToolCallRequest::new("read_file", params, &DefaultClock),
        )
        .await
}

pub async fn call_read_file_expecting_error(
    ctx: &RequestContext,
    discovery: &TestDiscoveryService,
    params: serde_json::Value,
) -> ToolDiscoveryRoutingServiceError {
    call_read_file(ctx, discovery, params)
        .await
        .expect_err("expected call_tool to return an error")
}

pub fn setup_success_result(host: &InMemoryMcpServerHost) -> Result<()> {
    host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;
    Ok(())
}

pub fn assert_single_audit_stderr_path(
    catalog: &InMemoryToolCatalog,
    expected_some: bool,
) -> Result<()> {
    let audits = catalog.audit_records()?;
    eyre::ensure!(audits.len() == 1, "expected 1 audit record, got {}", audits.len());
    let record = audits
        .first()
        .ok_or_else(|| eyre::eyre!("expected at least one audit record"))?;
    eyre::ensure!(
        record.stderr_log_path().is_some() == expected_some,
        "expected stderr_log_path().is_some() == {expected_some}"
    );
    Ok(())
}

/// Builds a discovery service wired to a custom policy adapter.
///
/// Returns both the service and the catalogue for test assertions.
#[expect(
    clippy::type_complexity,
    reason = "Generic Pol prevents a meaningful type alias for the return tuple"
)]
pub fn discovery_with_policy<Pol: crate::tool_registry::ports::ToolPolicyEnforcer + 'static>(
    registry: &Arc<InMemoryMcpServerRegistry>,
    host: &Arc<InMemoryMcpServerHost>,
    policy: Pol,
    clock: &Arc<DefaultClock>,
) -> (
    ToolDiscoveryRoutingService<
        InMemoryToolCatalog,
        InMemoryMcpServerRegistry,
        InMemoryMcpServerHost,
        Pol,
        ObjectStoreLogAdapter,
        DefaultClock,
    >,
    Arc<InMemoryToolCatalog>,
) {
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let service = ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: catalog.clone(),
            registry: registry.clone(),
            host: host.clone(),
            policy: Arc::new(policy),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock.clone(),
    );
    (service, catalog)
}
