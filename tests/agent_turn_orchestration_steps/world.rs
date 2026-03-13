//! Shared world state for agent turn orchestration BDD scenarios.

use std::collections::HashMap;
use std::sync::Arc;

use corbusier::agent_backend::{
    adapters::memory::{
        InMemoryAgentRuntime, InMemoryBackendRegistry, InMemoryToolRouter,
        InMemoryTurnSessionRepository,
    },
    domain::{BackendId, TurnSessionId},
    services::{
        AgentTurnOrchestrationError, AgentTurnOrchestratorConfig, AgentTurnOrchestratorPorts,
        AgentTurnOrchestratorService, ExecuteAgentTurnResponse,
    },
};
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use mockable::DefaultClock;
use rstest::fixture;
use uuid::Uuid;

/// Service type used by this BDD world.
pub type TestOrchestrator = AgentTurnOrchestratorService<
    InMemoryBackendRegistry,
    InMemoryTurnSessionRepository,
    InMemoryAgentRuntime,
    InMemoryToolRouter,
    DefaultClock,
>;

/// Scenario world for agent turn orchestration behaviour tests.
pub struct AgentTurnWorld {
    /// The backend registry repository.
    pub backend_registry: Arc<InMemoryBackendRegistry>,
    /// The turn-session repository.
    pub session_repository: Arc<InMemoryTurnSessionRepository>,
    /// In-memory runtime adapter.
    pub runtime: Arc<InMemoryAgentRuntime>,
    /// In-memory tool router adapter.
    pub tool_router: Arc<InMemoryToolRouter>,
    /// Orchestrator service under test.
    pub service: TestOrchestrator,
    /// Registered backend selected for the scenario.
    pub backend_id: Option<BackendId>,
    /// Conversation ID lookup by scenario label.
    pub conversations: HashMap<String, Uuid>,
    /// Last turn execution result.
    pub last_result: Option<Result<ExecuteAgentTurnResponse, AgentTurnOrchestrationError>>,
    /// Results from concurrent turn executions.
    pub concurrent_results: Option<(
        Result<ExecuteAgentTurnResponse, AgentTurnOrchestrationError>,
        Result<ExecuteAgentTurnResponse, AgentTurnOrchestrationError>,
    )>,
    /// Existing session ID used by reuse/rotation scenarios.
    pub existing_session_id: Option<TurnSessionId>,
    /// Request context for tenant-scoped orchestration operations.
    pub ctx: RequestContext,
}

impl AgentTurnWorld {
    /// Creates a world with empty scenario state.
    #[must_use]
    pub fn new() -> Self {
        let backend_registry = Arc::new(InMemoryBackendRegistry::new());
        let session_repository = Arc::new(InMemoryTurnSessionRepository::new());
        let runtime = Arc::new(InMemoryAgentRuntime::new());
        let tool_router = Arc::new(InMemoryToolRouter::new());
        let config = AgentTurnOrchestratorConfig::default();

        let service = AgentTurnOrchestratorService::with_config(
            AgentTurnOrchestratorPorts {
                backend_registry: backend_registry.clone(),
                turn_sessions: session_repository.clone(),
                runtime: runtime.clone(),
                tool_router: tool_router.clone(),
                clock: Arc::new(DefaultClock),
            },
            config,
        );

        Self {
            backend_registry,
            session_repository,
            runtime,
            tool_router,
            service,
            backend_id: None,
            conversations: HashMap::new(),
            last_result: None,
            concurrent_results: None,
            existing_session_id: None,
            ctx: RequestContext::new(
                TenantId::new(),
                CorrelationId::new(),
                UserId::new(),
                SessionId::new(),
            ),
        }
    }

    /// Returns a stable per-scenario conversation ID for a label.
    #[must_use]
    pub fn conversation_id(&mut self, label: &str) -> Uuid {
        *self
            .conversations
            .entry(label.to_owned())
            .or_insert_with(Uuid::new_v4)
    }
}

impl Default for AgentTurnWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture that creates a new scenario world.
#[fixture]
pub fn world() -> AgentTurnWorld {
    AgentTurnWorld::default()
}

/// Runs an async operation within synchronous step definitions.
///
/// This uses `tokio::task::block_in_place` with
/// `tokio::runtime::Handle::current().block_on(...)`, which panics when no
/// multi-threaded Tokio runtime is active (including current-thread runtimes).
/// Callers must invoke `run_async` only from within a multi-threaded runtime.
pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
