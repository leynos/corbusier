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

pub type HookPolicyHookEngine = HookEngineService<
    InMemoryHookDefinitionRepository,
    InMemoryHookActionExecutor,
    InMemoryHookExecutionLogRepository,
    InMemoryHookPolicyAuditRepository,
    DefaultClock,
>;

pub type HookPolicyGovernance =
    HookBackedToolExecutionGovernance<HookPolicyHookEngine, InMemoryHookPolicyAuditRepository>;

pub type HookPolicyLifecycleService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

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
    pub request_ctx: RequestContext,
    pub host: Arc<InMemoryMcpServerHost>,
    pub lifecycle: HookPolicyLifecycleService,
    pub discovery: HookPolicyDiscoveryService,
    pub definition_repo: InMemoryHookDefinitionRepository,
    pub action_executor: InMemoryHookActionExecutor,
    pub policy_audit: InMemoryHookPolicyAuditRepository,
    pub last_request: Option<ToolCallRequest>,
    pub last_result: Option<ToolCallResult>,
    pub last_error: Option<ToolDiscoveryRoutingServiceError>,
    pub last_task_id: Option<TaskId>,
    pub last_conversation_id: Option<ConversationId>,
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
        let governance = HookBackedToolExecutionGovernance::new(
            Arc::new(hook_engine),
            Arc::new(policy_audit.clone()),
        );
        let discovery = ToolDiscoveryRoutingService::new(
            ServicePorts {
                catalog,
                registry,
                host: host.clone(),
                policy: Arc::new(governance),
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

#[fixture]
pub fn world() -> HookPolicyWorld {
    HookPolicyWorld::default()
}

pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

pub fn stdio_request(
    name: &str,
) -> Result<RegisterMcpServerRequest, corbusier::tool_registry::domain::ToolRegistryDomainError> {
    Ok(RegisterMcpServerRequest::new(
        name,
        corbusier::tool_registry::domain::McpTransport::stdio("mcp-server")?,
    ))
}
