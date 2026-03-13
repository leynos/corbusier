//! Shared fixtures and helpers for turn orchestration unit tests.

use std::sync::Arc;

use crate::agent_backend::{
    adapters::memory::{
        InMemoryAgentRuntime, InMemoryBackendRegistry, InMemoryToolRouter,
        InMemoryTurnSessionRepository,
    },
    domain::{AgentBackendRegistration, AgentCapabilities, BackendId, BackendInfo, BackendName},
    ports::BackendRegistryRepository,
    services::{
        AgentTurnOrchestratorConfig, AgentTurnOrchestratorPorts, AgentTurnOrchestratorService,
    },
};
use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use mockable::DefaultClock;
use rstest::fixture;

pub type TestOrchestrator = AgentTurnOrchestratorService<
    InMemoryBackendRegistry,
    InMemoryTurnSessionRepository,
    InMemoryAgentRuntime,
    InMemoryToolRouter,
    DefaultClock,
>;

pub struct OrchestrationContext {
    pub backend_registry: Arc<InMemoryBackendRegistry>,
    pub session_repository: Arc<InMemoryTurnSessionRepository>,
    pub runtime: Arc<InMemoryAgentRuntime>,
    pub tool_router: Arc<InMemoryToolRouter>,
    pub service: TestOrchestrator,
    pub clock: Arc<DefaultClock>,
    pub ctx: RequestContext,
}

#[fixture]
pub fn context() -> OrchestrationContext {
    let backend_registry = Arc::new(InMemoryBackendRegistry::new());
    let session_repository = Arc::new(InMemoryTurnSessionRepository::new());
    let runtime = Arc::new(InMemoryAgentRuntime::new());
    let tool_router = Arc::new(InMemoryToolRouter::new());
    let clock = Arc::new(DefaultClock);
    let config = AgentTurnOrchestratorConfig::default();

    let service = AgentTurnOrchestratorService::with_config(
        AgentTurnOrchestratorPorts {
            backend_registry: backend_registry.clone(),
            turn_sessions: session_repository.clone(),
            runtime: runtime.clone(),
            tool_router: tool_router.clone(),
            clock: clock.clone(),
        },
        config,
    );

    OrchestrationContext {
        backend_registry,
        session_repository,
        runtime,
        tool_router,
        service,
        clock,
        ctx: RequestContext::new(
            TenantId::new(),
            CorrelationId::new(),
            UserId::new(),
            SessionId::new(),
        ),
    }
}

pub fn create_backend_registration(
    name: &str,
    clock: &DefaultClock,
) -> Result<AgentBackendRegistration, eyre::Report> {
    let backend_name = BackendName::new(name)?;
    let capabilities = AgentCapabilities::new(true, true);
    let info = BackendInfo::new(name, "1.0.0", "test-provider")?;
    Ok(AgentBackendRegistration::new(
        backend_name,
        capabilities,
        info,
        clock,
    ))
}

pub async fn register_backend(
    context: &OrchestrationContext,
    name: &str,
) -> Result<BackendId, eyre::Report> {
    let registration = create_backend_registration(name, context.clock.as_ref())?;
    let backend_id = registration.id();
    context
        .backend_registry
        .register(&context.ctx, &registration)
        .await?;
    Ok(backend_id)
}
