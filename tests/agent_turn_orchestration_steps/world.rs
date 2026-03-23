//! Shared world state for agent turn orchestration BDD scenarios.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use corbusier::agent_backend::{
    adapters::memory::{
        InMemoryAgentRuntime, InMemoryBackendRegistry, InMemoryToolRouter,
        InMemoryTurnSessionRepository,
    },
    domain::{BackendId, TurnSessionId},
    services::{AgentTurnOrchestrationError, ExecuteAgentTurnResponse},
};
use corbusier::context::RequestContext;
use corbusier::test_support::{InMemoryAgentTurnOrchestrator, build_in_memory_orchestrator};
use rstest::fixture;
use uuid::Uuid;

/// Service type used by this BDD world.
pub type TestOrchestrator = InMemoryAgentTurnOrchestrator;

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
        let stack = build_in_memory_orchestrator();

        Self {
            backend_registry: stack.backend_registry,
            session_repository: stack.session_repository,
            runtime: stack.runtime,
            tool_router: stack.tool_router,
            service: stack.service,
            backend_id: None,
            conversations: HashMap::new(),
            last_result: None,
            concurrent_results: None,
            existing_session_id: None,
            ctx: stack.ctx,
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

/// Semantic wrapper for an assistant text response in BDD steps.
#[derive(Debug, Clone)]
pub struct AssistantText(pub String);

impl std::str::FromStr for AssistantText {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

/// Semantic wrapper for a backend name label in BDD steps.
#[derive(Debug, Clone)]
pub struct BackendNameLabel(pub String);

impl std::str::FromStr for BackendNameLabel {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

/// Semantic wrapper for a tool name in BDD steps.
#[derive(Debug, Clone)]
pub struct ToolName(pub String);

impl std::str::FromStr for ToolName {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

/// Semantic wrapper for a conversation label (map key) in BDD steps.
#[derive(Debug, Clone)]
pub struct ConversationLabel(pub String);

impl std::str::FromStr for ConversationLabel {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

/// Semantic wrapper for a tool audit status in BDD steps.
#[derive(Debug, Clone)]
pub struct AuditStatusLabel(String);

impl AuditStatusLabel {
    /// Returns the wrapped audit-status label.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for AuditStatusLabel {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
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
