//! Shared world state for hook-backed tool policy enforcement scenarios.

use std::sync::Arc;

use corbusier::context::RequestContext;
use corbusier::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository, InMemoryHookPolicyAuditRepository,
};
use corbusier::hook_engine::domain::PolicyAuditEvent;
use corbusier::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use corbusier::message::domain::ConversationId;
use corbusier::test_support::test_request_ctx;
use corbusier::tool_registry::adapters::{
    HookBackedToolExecutionGovernance, InMemoryMcpServerHost, ObjectStoreLogAdapter,
    memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
};
use corbusier::tool_registry::domain::{LogRetentionPolicy, ToolCallRequest, ToolCallResult};
use corbusier::tool_registry::services::{
    McpServerLifecycleService, ServicePorts, ToolDiscoveryRoutingService,
    ToolDiscoveryRoutingServiceError,
};
use corbusier::{task::domain::TaskId, tool_registry::services::RegisterMcpServerRequest};
use mockable::DefaultClock;
use rstest::fixture;

/// Hook engine alias used by hook-backed policy enforcement scenarios.
pub type HookPolicyHookEngine = HookEngineService<
    InMemoryHookDefinitionRepository,
    InMemoryHookActionExecutor,
    InMemoryHookExecutionLogRepository,
    InMemoryHookPolicyAuditRepository,
    DefaultClock,
>;

/// Governance adapter alias that evaluates tool calls through the hook engine.
pub type HookPolicyGovernance = HookBackedToolExecutionGovernance<HookPolicyHookEngine>;

/// Lifecycle service alias used by hook-policy BDD scenarios.
pub type HookPolicyLifecycleService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

/// Discovery service alias wired to hook-backed governance for BDD scenarios.
pub type HookPolicyDiscoveryService = ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    HookPolicyGovernance,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

/// Scenario world for hook-backed policy enforcement behaviour.
pub struct HookPolicyWorld {
    /// Request context used for lifecycle, discovery, and governance calls.
    pub request_ctx: RequestContext,
    /// In-memory MCP host used to simulate server behaviour during scenarios.
    pub host: Arc<InMemoryMcpServerHost>,
    /// Lifecycle service used to register, start, and stop test servers.
    pub lifecycle: HookPolicyLifecycleService,
    /// Discovery service under test, wired to hook-backed governance.
    pub discovery: HookPolicyDiscoveryService,
    /// Hook definition repository populated by scenario setup steps.
    pub definition_repo: InMemoryHookDefinitionRepository,
    /// Action executor seeded with hook policy outputs for each scenario.
    pub action_executor: InMemoryHookActionExecutor,
    /// Policy audit repository queried by assertions after tool execution.
    pub policy_audit: InMemoryHookPolicyAuditRepository,
    /// Most recent tool call request issued during a scenario, if any.
    pub last_request: Option<ToolCallRequest>,
    /// Most recent successful tool call result recorded by a scenario step.
    pub last_result: Option<ToolCallResult>,
    /// Most recent discovery or governance error captured by a scenario step.
    pub last_error: Option<ToolDiscoveryRoutingServiceError>,
    /// Task identifier associated with the current scenario request, if set.
    pub last_task_id: Option<TaskId>,
    /// Conversation identifier associated with the current scenario request, if
    /// set.
    pub last_conversation_id: Option<ConversationId>,
    /// Policy audit events captured for the most recent scenario assertion.
    pub last_events: Vec<PolicyAuditEvent>,
}

impl HookPolicyWorld {
    /// Creates a world with hook-backed governance wired into discovery.
    #[must_use]
    pub fn new() -> Self {
        let request_ctx = test_request_ctx();
        let registry = Arc::new(InMemoryMcpServerRegistry::new());
        let host = Arc::new(InMemoryMcpServerHost::new());
        let catalog = Arc::new(InMemoryToolCatalog::new());
        let clock = Arc::new(DefaultClock);
        let lifecycle =
            McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());

        let definition_repo = InMemoryHookDefinitionRepository::new();
        let action_executor = InMemoryHookActionExecutor::new();
        let execution_log = InMemoryHookExecutionLogRepository::new();
        let policy_audit = InMemoryHookPolicyAuditRepository::new();
        let hook_engine = HookEngineService::new(HookEngineServiceDeps {
            definition_repository: Arc::new(definition_repo.clone()),
            action_executor: Arc::new(action_executor.clone()),
            execution_log: Arc::new(execution_log),
            policy_audit_repository: Arc::new(policy_audit.clone()),
            clock: clock.clone(),
        });
        let governance = HookBackedToolExecutionGovernance::new(Arc::new(hook_engine));
        let discovery = ToolDiscoveryRoutingService::new(
            ServicePorts {
                catalog,
                registry,
                host: host.clone(),
                governance: Arc::new(governance),
                log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
            },
            LogRetentionPolicy::default(),
            clock,
        );

        Self {
            request_ctx,
            host,
            lifecycle,
            discovery,
            definition_repo,
            action_executor,
            policy_audit,
            last_request: None,
            last_result: None,
            last_error: None,
            last_task_id: None,
            last_conversation_id: None,
            last_events: Vec::new(),
        }
    }
}

impl Default for HookPolicyWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds the default hook-policy BDD world fixture.
#[fixture]
pub fn world() -> HookPolicyWorld {
    HookPolicyWorld::default()
}

/// Runs an async operation to completion, reusing the current runtime when one
/// is already active and otherwise creating a dedicated current-thread runtime.
pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> Result<T, std::io::Error> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        return match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                Ok(tokio::task::block_in_place(|| handle.block_on(future)))
            }
            tokio::runtime::RuntimeFlavor::CurrentThread => Err(std::io::Error::other(
                "cannot block_on within a current-thread Tokio runtime",
            )),
            _ => Err(std::io::Error::other(
                "unsupported Tokio runtime flavour for blocking async helper",
            )),
        };
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map(|runtime| runtime.block_on(future))
}

/// Creates a stdio registration request for the hook-policy test server.
pub fn stdio_request(
    name: &str,
) -> Result<RegisterMcpServerRequest, corbusier::tool_registry::domain::ToolRegistryDomainError> {
    Ok(RegisterMcpServerRequest::new(
        name,
        corbusier::tool_registry::domain::McpTransport::stdio("mcp-server")?,
    ))
}
