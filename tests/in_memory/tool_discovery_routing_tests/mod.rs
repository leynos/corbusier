//! In-memory integration tests for tool discovery and call routing.
//!
//! Tests are split by concern:
//! - `call_routing_tests`: Multi-server routing and ambiguity detection
//! - `discovery_lifecycle_tests`: Discovery, availability, restart, audit, stderr

mod call_routing_tests;
mod discovery_lifecycle_tests;
mod policy_enforcement_tests;

use std::sync::Arc;

pub use super::helpers::request_ctx;
use corbusier::context::RequestContext;
use corbusier::tool_registry::{
    adapters::{
        AllowAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter,
        memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
    },
    domain::{
        LogRetentionPolicy, McpServerName, McpToolDefinition, McpTransport, ToolRegistryDomainError,
    },
    services::{
        McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
        ToolDiscoveryRoutingService,
    },
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::fixture;
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

pub struct IntegrationContext {
    pub registry: Arc<InMemoryMcpServerRegistry>,
    pub host: Arc<InMemoryMcpServerHost>,
    pub lifecycle: TestLifecycleService,
    pub discovery: TestDiscoveryService,
    pub catalog: Arc<InMemoryToolCatalog>,
}

#[fixture]
pub fn integration_ctx() -> IntegrationContext {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let clock = Arc::new(DefaultClock);

    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let discovery = ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: catalog.clone(),
            registry: registry.clone(),
            host: host.clone(),
            policy: Arc::new(AllowAllPolicy),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock,
    );

    IntegrationContext {
        registry,
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
        .discover_and_persist_tools(request_ctx, registered.id())
        .await?;
    Ok(registered.id())
}
