//! Shared fixtures and helpers for orchestration integration tests.

use std::sync::Arc;

use chrono::Duration;
use corbusier::agent_backend::{
    adapters::{
        memory::{InMemoryAgentRuntime, InMemoryToolRouter},
        postgres::{BackendPgPool, PostgresBackendRegistry, PostgresTurnSessionRepository},
    },
    services::{
        AgentTurnOrchestratorConfig, AgentTurnOrchestratorPorts, AgentTurnOrchestratorService,
        BackendRegistryService, RegisterBackendRequest,
    },
};
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use corbusier::message::domain::ConversationId;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::fixture;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, insert_conversation, postgres_cluster,
};

pub type TestOrchestrator = AgentTurnOrchestratorService<
    PostgresBackendRegistry,
    PostgresTurnSessionRepository,
    InMemoryAgentRuntime,
    InMemoryToolRouter,
    DefaultClock,
>;

pub type TestRegistryService = BackendRegistryService<PostgresBackendRegistry, DefaultClock>;

pub struct OrchestrationContext {
    pub cluster: PostgresCluster,
    pub ctx: RequestContext,
    pub backend_registry: Arc<PostgresBackendRegistry>,
    pub service: TestOrchestrator,
    pub registry_service: TestRegistryService,
    pub session_repository: Arc<PostgresTurnSessionRepository>,
    pub runtime: Arc<InMemoryAgentRuntime>,
    pub router: Arc<InMemoryToolRouter>,
    pub clock: Arc<DefaultClock>,
    pub temp_db: TemporaryDatabase,
}

async fn setup_context(cluster: PostgresCluster) -> Result<OrchestrationContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("turn_orch_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;
    let manager = ConnectionManager::<PgConnection>::new(db.url().to_owned());
    let pool: BackendPgPool = diesel::r2d2::Pool::builder()
        .max_size(5)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)?;

    let backend_registry = Arc::new(PostgresBackendRegistry::new(pool.clone()));
    let session_repository = Arc::new(PostgresTurnSessionRepository::new(pool));
    let runtime = Arc::new(InMemoryAgentRuntime::new());
    let router = Arc::new(InMemoryToolRouter::new());
    let clock = Arc::new(DefaultClock);
    let config = AgentTurnOrchestratorConfig::new(Duration::minutes(5))
        .map_err(|err| Box::new(err) as BoxError)?;

    let service = AgentTurnOrchestratorService::with_config(
        AgentTurnOrchestratorPorts {
            backend_registry: backend_registry.clone(),
            turn_sessions: session_repository.clone(),
            runtime: runtime.clone(),
            tool_router: router.clone(),
            clock: clock.clone(),
        },
        config,
    );

    let registry_service = BackendRegistryService::new(backend_registry.clone(), clock.clone());

    Ok(OrchestrationContext {
        cluster,
        ctx: RequestContext::new(
            TenantId::new(),
            CorrelationId::new(),
            UserId::new(),
            SessionId::new(),
        ),
        backend_registry,
        service,
        registry_service,
        session_repository,
        runtime,
        router,
        clock,
        temp_db: db,
    })
}

pub async fn ensure_conversation_exists(
    context: &OrchestrationContext,
    conversation_id: Uuid,
) -> Result<(), BoxError> {
    insert_conversation(
        context.cluster,
        context.temp_db.name(),
        ConversationId::from_uuid(conversation_id),
    )
    .await
}

pub async fn register_backend(
    context: &OrchestrationContext,
    name: &str,
) -> Result<Uuid, BoxError> {
    let backend = context
        .registry_service
        .register(
            &context.ctx,
            RegisterBackendRequest::new(name, name, "1.0.0", "test-provider")
                .with_capabilities(true, true),
        )
        .await
        .map_err(|err| Box::new(err) as BoxError)?;
    Ok(backend.id().into_inner())
}

pub fn other_tenant_context(ctx: &RequestContext) -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        ctx.correlation_id(),
        ctx.user_id(),
        ctx.session_id(),
    )
}

#[fixture]
pub async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<OrchestrationContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_context(cluster).await
}
