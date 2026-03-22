//! Shared fixtures and helpers for turn orchestration unit tests.

use crate::agent_backend::{
    domain::{AgentBackendRegistration, AgentCapabilities, BackendId, BackendInfo, BackendName},
    ports::BackendRegistryRepository,
};
use crate::context::RequestContext;
use crate::test_support::{InMemoryAgentTurnOrchestrator, build_in_memory_orchestrator};
use mockable::DefaultClock;
use rstest::fixture;

pub type TestOrchestrator = InMemoryAgentTurnOrchestrator;

pub struct OrchestrationContext {
    pub backend_registry:
        std::sync::Arc<crate::agent_backend::adapters::memory::InMemoryBackendRegistry>,
    pub session_repository:
        std::sync::Arc<crate::agent_backend::adapters::memory::InMemoryTurnSessionRepository>,
    pub runtime: std::sync::Arc<crate::agent_backend::adapters::memory::InMemoryAgentRuntime>,
    pub tool_router: std::sync::Arc<crate::agent_backend::adapters::memory::InMemoryToolRouter>,
    pub service: TestOrchestrator,
    pub clock: std::sync::Arc<DefaultClock>,
    pub ctx: RequestContext,
}

#[fixture]
pub fn context() -> OrchestrationContext {
    let stack = build_in_memory_orchestrator();

    OrchestrationContext {
        backend_registry: stack.backend_registry,
        session_repository: stack.session_repository,
        runtime: stack.runtime,
        tool_router: stack.tool_router,
        service: stack.service,
        clock: stack.clock,
        ctx: stack.ctx,
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
