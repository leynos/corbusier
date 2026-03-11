//! `PostgreSQL` integration tests for turn orchestration and session continuity.

use std::sync::Arc;

use chrono::{Duration, Utc};
use corbusier::agent_backend::{
    adapters::{
        memory::{InMemoryAgentRuntime, InMemoryToolRouter},
        postgres::{BackendPgPool, PostgresBackendRegistry, PostgresTurnSessionRepository},
    },
    domain::{
        PersistedTurnSessionData, RuntimeSessionId, ToolCallRequest, TurnExecutionRequest,
        TurnExecutionResult, TurnSession, TurnSessionStatus,
    },
    ports::TurnSessionRepository,
    services::{
        AgentTurnOrchestrationError, AgentTurnOrchestratorConfig, AgentTurnOrchestratorPorts,
        AgentTurnOrchestratorService, BackendRegistryService, ExecuteAgentTurnRequest,
        RegisterBackendRequest,
    },
};
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use corbusier::message::domain::ConversationId;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, insert_conversation, postgres_cluster,
};

type TestOrchestrator = AgentTurnOrchestratorService<
    PostgresBackendRegistry,
    PostgresTurnSessionRepository,
    InMemoryAgentRuntime,
    InMemoryToolRouter,
    DefaultClock,
>;

type TestRegistryService = BackendRegistryService<PostgresBackendRegistry, DefaultClock>;

struct OrchestrationContext {
    cluster: PostgresCluster,
    ctx: RequestContext,
    service: TestOrchestrator,
    registry_service: TestRegistryService,
    session_repository: Arc<PostgresTurnSessionRepository>,
    runtime: Arc<InMemoryAgentRuntime>,
    router: Arc<InMemoryToolRouter>,
    temp_db: TemporaryDatabase,
}

async fn setup_context(cluster: PostgresCluster) -> Result<OrchestrationContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("turn_orch_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;
    let manager = ConnectionManager::<PgConnection>::new(db.url().to_owned());
    let pool: BackendPgPool = diesel::r2d2::Pool::builder()
        .max_size(1)
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

    let registry_service = BackendRegistryService::new(backend_registry, clock);

    Ok(OrchestrationContext {
        cluster,
        ctx: RequestContext::new(
            TenantId::new(),
            CorrelationId::new(),
            UserId::new(),
            SessionId::new(),
        ),
        service,
        registry_service,
        session_repository,
        runtime,
        router,
        temp_db: db,
    })
}

async fn ensure_conversation_exists(
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

async fn register_backend(context: &OrchestrationContext, name: &str) -> Result<Uuid, BoxError> {
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

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<OrchestrationContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_context(cluster).await
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_orchestrates_turn_and_reuses_session(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new(
            "first",
            vec![
                ToolCallRequest::new("lookup", json!({"q": "docs"}))
                    .map_err(|err| Box::new(err) as BoxError)?,
            ],
        ))
        .map_err(|err| Box::new(err) as BoxError)?;
    ctx.router
        .set_tool_response("lookup", json!({"matches": 2}))
        .map_err(|err| Box::new(err) as BoxError)?;

    let first = ctx
        .service
        .execute_turn(
            &ctx.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "first", Vec::new()),
            ),
        )
        .await
        .map_err(|err| Box::new(err) as BoxError)?;

    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new("second", Vec::new()))
        .map_err(|err| Box::new(err) as BoxError)?;

    let second = ctx
        .service
        .execute_turn(
            &ctx.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "second", Vec::new()),
            ),
        )
        .await
        .map_err(|err| Box::new(err) as BoxError)?;

    assert!(!first.reused_session());
    assert!(second.reused_session());
    assert_eq!(first.session_id(), second.session_id());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_rotates_expired_session(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "codex_cli").await?,
    );
    let conversation_id = Uuid::new_v4();
    let now = Utc::now();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    let expired_session = TurnSession::from_persisted(PersistedTurnSessionData {
        id: corbusier::agent_backend::domain::TurnSessionId::new(),
        backend_id,
        conversation_id,
        runtime_session_id: RuntimeSessionId::new("expired-runtime")
            .map_err(|err| Box::new(err) as BoxError)?,
        status: TurnSessionStatus::Active,
        ttl_seconds: 45,
        started_at: now - Duration::seconds(90),
        last_used_at: now - Duration::seconds(90),
        expires_at: now - Duration::seconds(1),
        ended_at: None,
        turn_count: 2,
    });
    ctx.session_repository
        .upsert_session(&expired_session)
        .await
        .map_err(|err| Box::new(err) as BoxError)?;

    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new("rotated", Vec::new()))
        .map_err(|err| Box::new(err) as BoxError)?;

    let response = ctx
        .service
        .execute_turn(
            &ctx.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "rotate", Vec::new()),
            ),
        )
        .await
        .map_err(|err| Box::new(err) as BoxError)?;

    assert!(response.rotated_session());
    assert_ne!(response.runtime_session_id(), "expired-runtime");

    let active = ctx
        .session_repository
        .find_active_session(backend_id, conversation_id)
        .await
        .map_err(|err| Box::new(err) as BoxError)?
        .ok_or_else(|| {
            Box::new(std::io::Error::other("expected active replacement session")) as BoxError
        })?;

    assert_ne!(active.id(), expired_session.id());
    assert_eq!(active.status(), TurnSessionStatus::Active);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_serializes_concurrent_calls_for_same_session_key(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new("first", Vec::new()))
        .map_err(|err| Box::new(err) as BoxError)?;
    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new("second", Vec::new()))
        .map_err(|err| Box::new(err) as BoxError)?;

    let first_request = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "1", Vec::new()),
    );
    let second_request = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "2", Vec::new()),
    );

    let (first_result, second_result) = tokio::join!(
        ctx.service.execute_turn(&ctx.ctx, first_request),
        ctx.service.execute_turn(&ctx.ctx, second_request)
    );

    let first_response = first_result.map_err(|err| Box::new(err) as BoxError)?;
    let second_response = second_result.map_err(|err| Box::new(err) as BoxError)?;
    assert_eq!(first_response.session_id(), second_response.session_id());
    let reused_count = [
        first_response.reused_session(),
        second_response.reused_session(),
    ]
    .into_iter()
    .filter(|is_reused| *is_reused)
    .count();
    assert_eq!(
        reused_count, 1,
        "expected exactly one concurrent call to reuse the created session"
    );

    let created_sessions = ctx
        .runtime
        .created_session_ids()
        .map_err(|err| Box::new(err) as BoxError)?;
    assert_eq!(created_sessions.len(), 1);

    let active = ctx
        .session_repository
        .find_active_session(backend_id, conversation_id)
        .await
        .map_err(|err| Box::new(err) as BoxError)?
        .ok_or_else(|| Box::new(std::io::Error::other("expected active session")) as BoxError)?;
    assert_eq!(active.turn_count(), 2);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_propagates_tool_routing_failure(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&ctx, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new(
            "failure",
            vec![
                ToolCallRequest::new("will_fail", json!({"arg": true}))
                    .map_err(|err| Box::new(err) as BoxError)?,
            ],
        ))
        .map_err(|err| Box::new(err) as BoxError)?;
    ctx.router
        .fail_tool("will_fail", "simulated failure")
        .map_err(|err| Box::new(err) as BoxError)?;

    let result = ctx
        .service
        .execute_turn(
            &ctx.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "fail", Vec::new()),
            ),
        )
        .await;

    assert!(matches!(
        result,
        Err(AgentTurnOrchestrationError::ToolRouting { .. })
    ));
    Ok(())
}
