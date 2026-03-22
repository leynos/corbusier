//! Shared test infrastructure for tool discovery and routing tests.

pub use crate::test_support::test_request_ctx;
use crate::{
    context::RequestContext,
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

/// In-memory lifecycle service used by discovery tests.
pub type TestLifecycleService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

/// In-memory discovery service used by end-to-end routing tests.
pub type TestDiscoveryService = ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    AllowAllPolicy,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

/// Test bundle wiring together in-memory tool-registry collaborators.
pub struct TestBundle {
    /// In-memory host fake used to control tool catalogues and call results.
    pub host: Arc<InMemoryMcpServerHost>,
    /// Lifecycle service backed by in-memory registry and host adapters.
    pub lifecycle: TestLifecycleService,
    /// Discovery service backed by the in-memory catalogue and host adapters.
    pub discovery: TestDiscoveryService,
    /// In-memory catalogue exposed for direct test assertions.
    pub catalog: Arc<InMemoryToolCatalog>,
}

/// Builds the standard in-memory bundle for discovery tests.
#[fixture]
pub fn bundle() -> TestBundle {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let clock = Arc::new(DefaultClock);
    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let (discovery, catalog) = discovery_with_policy(&registry, &host, AllowAllPolicy, &clock);
    TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
    }
}

/// Creates a stdio registration request for an in-memory test server.
pub fn stdio_request(name: &str) -> Result<RegisterMcpServerRequest, ToolRegistryDomainError> {
    Ok(RegisterMcpServerRequest::new(
        name,
        McpTransport::stdio("mcp-server")?,
    ))
}

/// Builds the canonical `read_file` tool definition used across tests.
pub fn read_file_tool() -> Result<McpToolDefinition> {
    Ok(McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    )?)
}

/// Registers, starts, and discovers the default in-memory test server.
///
/// NOTE: The helper is test-only and assumes `workspace_tools` semantics.
pub async fn register_start_discover<Gov: crate::tool_registry::ports::ToolExecutionGovernance>(
    host: &InMemoryMcpServerHost,
    lifecycle: &TestLifecycleService,
    discovery: &ToolDiscoveryRoutingService<
        InMemoryToolCatalog,
        InMemoryMcpServerRegistry,
        InMemoryMcpServerHost,
        Gov,
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
/// Service triplet used by helpers that are generic over the policy
/// adapter.
pub struct TestServices<'a, Gov: crate::tool_registry::ports::ToolExecutionGovernance> {
    /// The in-memory host adapter.
    pub host: &'a InMemoryMcpServerHost,
    /// The lifecycle service.
    pub lifecycle: &'a TestLifecycleService,
    /// The discovery service.
    pub discovery: &'a PolicyDiscoveryService<Gov>,
}

/// Registers a server, starts it, configures startup stderr, discovers
/// tools, and returns the server identifier and captured startup stderr.
pub async fn register_start_with_stderr<
    Gov: crate::tool_registry::ports::ToolExecutionGovernance,
>(
    services: &TestServices<'_, Gov>,
    ctx: &RequestContext,
    startup_stderr: bytes::Bytes,
) -> Result<(
    crate::tool_registry::domain::McpServerId,
    Option<bytes::Bytes>,
)> {
    services.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    services
        .host
        .set_startup_stderr(McpServerName::new("workspace_tools")?, startup_stderr)?;
    let registered = services
        .lifecycle
        .register(ctx, stdio_request("workspace_tools")?)
        .await?;
    let start_result = services.lifecycle.start(ctx, registered.id()).await?;
    services
        .discovery
        .discover_and_persist_tools(ctx, registered.id())
        .await?;
    Ok((registered.id(), start_result.startup_stderr))
}

/// Calls the canonical `read_file` tool through the discovery service.
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

/// Calls `read_file` and returns the expected routing error.
pub async fn call_read_file_expecting_error(
    ctx: &RequestContext,
    discovery: &TestDiscoveryService,
    params: serde_json::Value,
) -> Result<ToolDiscoveryRoutingServiceError> {
    match call_read_file(ctx, discovery, params).await {
        Ok(_) => Err(eyre::eyre!("expected call_tool to return an error")),
        Err(err) => Ok(err),
    }
}

/// Configures the host to return a successful `read_file` result.
pub fn setup_success_result(host: &InMemoryMcpServerHost) -> Result<()> {
    host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;
    Ok(())
}

/// Asserts whether the single captured audit row includes a stderr log path.
pub fn assert_single_audit_stderr_path(
    catalog: &InMemoryToolCatalog,
    tenant_id: crate::context::TenantId,
    expected_some: bool,
) -> Result<()> {
    let audits = catalog.audit_records(tenant_id)?;
    eyre::ensure!(
        audits.len() == 1,
        "expected 1 audit record, got {}",
        audits.len()
    );
    let record = audits
        .first()
        .ok_or_else(|| eyre::eyre!("expected at least one audit record"))?;
    eyre::ensure!(
        record.stderr_log_path().is_some() == expected_some,
        "expected stderr_log_path().is_some() == {expected_some}"
    );
    Ok(())
}

/// Discovery service parameterized by a custom policy adapter.
pub type PolicyDiscoveryService<Gov> = ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    Gov,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

/// Builds a discovery service wired to a custom policy adapter.
///
/// Returns both the service and the catalogue for test assertions.
/// NOTE: This helper is intended for in-memory, test-only wiring.
pub fn discovery_with_policy<
    Gov: crate::tool_registry::ports::ToolExecutionGovernance + 'static,
>(
    registry: &Arc<InMemoryMcpServerRegistry>,
    host: &Arc<InMemoryMcpServerHost>,
    policy: Gov,
    clock: &Arc<DefaultClock>,
) -> (PolicyDiscoveryService<Gov>, Arc<InMemoryToolCatalog>) {
    build_discovery_service(registry, host, policy, clock)
}

fn build_discovery_service<Gov: crate::tool_registry::ports::ToolExecutionGovernance + 'static>(
    registry: &Arc<InMemoryMcpServerRegistry>,
    host: &Arc<InMemoryMcpServerHost>,
    policy: Gov,
    clock: &Arc<DefaultClock>,
) -> (PolicyDiscoveryService<Gov>, Arc<InMemoryToolCatalog>) {
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
