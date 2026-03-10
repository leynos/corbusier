//! Shared world state for tool discovery and routing BDD scenarios.

use std::sync::Arc;

use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use corbusier::tool_registry::{
    adapters::{
        InMemoryMcpServerHost,
        memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
    },
    domain::McpServerRegistration,
    services::ToolDiscoveryRoutingServiceError,
};
use eyre::eyre;
use mockable::DefaultClock;
use rstest::fixture;

use super::{TestDiscoveryService, TestLifecycleService};

/// Scenario world for tool discovery and routing behaviour tests.
pub struct ToolDiscoveryWorld {
    /// The in-memory MCP server host.
    pub host: Arc<InMemoryMcpServerHost>,
    /// Lifecycle service instance.
    pub lifecycle: Option<TestLifecycleService>,
    /// Discovery and routing service instance.
    pub discovery: Option<TestDiscoveryService>,
    /// In-memory catalogue for audit assertions.
    pub catalog: Arc<InMemoryToolCatalog>,
    /// Request context shared across all steps.
    pub request_ctx: RequestContext,
    /// Server name queued for registration.
    pub pending_name: Option<String>,
    /// Server command queued for registration.
    pub pending_command: Option<String>,
    /// Last successfully registered/started server.
    pub registered_server: Option<McpServerRegistration>,
    /// Whether the last tool call succeeded.
    pub last_call_succeeded: Option<bool>,
    /// Error from the last failed tool call.
    pub last_error: Option<ToolDiscoveryRoutingServiceError>,
}

impl ToolDiscoveryWorld {
    /// Creates a world with freshly wired services.
    #[must_use]
    pub fn new() -> Self {
        let registry = Arc::new(InMemoryMcpServerRegistry::new());
        let host = Arc::new(InMemoryMcpServerHost::new());
        let catalog = Arc::new(InMemoryToolCatalog::new());
        let clock = Arc::new(DefaultClock);

        let lifecycle = super::lifecycle_service(&registry, &host, &clock);
        let discovery = super::discovery_service(&catalog, &registry, &host, &clock);

        Self {
            host,
            lifecycle: Some(lifecycle),
            discovery: Some(discovery),
            catalog,
            request_ctx: RequestContext::new(
                TenantId::new(),
                CorrelationId::new(),
                UserId::new(),
                SessionId::new(),
            ),
            pending_name: None,
            pending_command: None,
            registered_server: None,
            last_call_succeeded: None,
            last_error: None,
        }
    }

    /// Returns a reference to the lifecycle service.
    pub fn lifecycle(&self) -> Result<&TestLifecycleService, eyre::Report> {
        self.lifecycle
            .as_ref()
            .ok_or_else(|| eyre!("lifecycle service should exist"))
    }

    /// Returns a reference to the discovery service.
    pub fn discovery(&self) -> Result<&TestDiscoveryService, eyre::Report> {
        self.discovery
            .as_ref()
            .ok_or_else(|| eyre!("discovery service should exist"))
    }

    /// Returns the pending server name.
    pub fn pending_name(&self) -> Result<&str, eyre::Report> {
        self.pending_name
            .as_deref()
            .ok_or_else(|| eyre!("pending server name should exist"))
    }
}

impl Default for ToolDiscoveryWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture that creates a new scenario world.
#[fixture]
pub fn world() -> ToolDiscoveryWorld {
    ToolDiscoveryWorld::default()
}
