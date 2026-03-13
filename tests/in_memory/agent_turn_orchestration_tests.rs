//! In-memory integration tests for agent turn orchestration and sessions.

use std::sync::Arc;

use chrono::{Duration, Utc};
use corbusier::agent_backend::{
    adapters::memory::{
        InMemoryAgentRuntime, InMemoryBackendRegistry, InMemoryToolRouter,
        InMemoryTurnSessionRepository,
    },
    domain::{
        PersistedTurnSessionData, RuntimeSessionId, ToolCallRequest, TurnExecutionRequest,
        TurnExecutionResult, TurnSession, TurnSessionStatus,
    },
    ports::TurnSessionRepository,
    services::{
        AgentTurnOrchestrationError, AgentTurnOrchestratorConfig, AgentTurnOrchestratorPorts,
        AgentTurnOrchestratorService, ExecuteAgentTurnRequest,
    },
};
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

type TestOrchestrator = AgentTurnOrchestratorService<
    InMemoryBackendRegistry,
    InMemoryTurnSessionRepository,
    InMemoryAgentRuntime,
    InMemoryToolRouter,
    DefaultClock,
>;

struct TestContext {
    backend_registry: Arc<InMemoryBackendRegistry>,
    session_repository: Arc<InMemoryTurnSessionRepository>,
    runtime: Arc<InMemoryAgentRuntime>,
    router: Arc<InMemoryToolRouter>,
    service: TestOrchestrator,
    ctx: RequestContext,
}

#[fixture]
fn context() -> TestContext {
    let backend_registry = Arc::new(InMemoryBackendRegistry::new());
    let session_repository = Arc::new(InMemoryTurnSessionRepository::new());
    let runtime = Arc::new(InMemoryAgentRuntime::new());
    let router = Arc::new(InMemoryToolRouter::new());
    let config = AgentTurnOrchestratorConfig::default();

    let service = AgentTurnOrchestratorService::with_config(
        AgentTurnOrchestratorPorts {
            backend_registry: backend_registry.clone(),
            turn_sessions: session_repository.clone(),
            runtime: runtime.clone(),
            tool_router: router.clone(),
            clock: Arc::new(DefaultClock),
        },
        config,
    );

    TestContext {
        backend_registry,
        session_repository,
        runtime,
        router,
        service,
        ctx: RequestContext::new(
            TenantId::new(),
            CorrelationId::new(),
            UserId::new(),
            SessionId::new(),
        ),
    }
}

async fn register_backend(context: &TestContext, name: &str) -> Result<uuid::Uuid, eyre::Report> {
    let request = corbusier::agent_backend::services::RegisterBackendRequest::new(
        name,
        name,
        "1.0.0",
        "test-provider",
    )
    .with_capabilities(true, true);

    let registry_service = corbusier::agent_backend::services::BackendRegistryService::new(
        context.backend_registry.clone(),
        Arc::new(DefaultClock),
    );
    let backend = registry_service.register(&context.ctx, request).await?;
    Ok(backend.id().into_inner())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn orchestrates_turn_and_reuses_session_before_expiry(
    context: TestContext,
) -> Result<(), eyre::Report> {
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&context, "claude_code_sdk").await?,
    );
    let conversation_id = Uuid::new_v4();

    context.runtime.queue_turn_result(TurnExecutionResult::new(
        "first-response",
        vec![ToolCallRequest::new("lookup", json!({"q": "roadmap"}))?],
    ))?;
    context
        .router
        .set_tool_response("lookup", json!({"result": "ok"}))?;

    let first = context
        .service
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "first", Vec::new()),
            ),
        )
        .await?;

    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("second-response", Vec::new()))?;

    let second = context
        .service
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "second", Vec::new()),
            ),
        )
        .await?;

    assert!(!first.reused_session());
    assert!(second.reused_session());
    assert_eq!(first.session_id(), second.session_id());
    assert!(second.tool_results().is_empty());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn rotates_expired_session_and_marks_prior_session_expired(
    context: TestContext,
) -> Result<(), eyre::Report> {
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&context, "codex_cli").await?,
    );
    let conversation_id = Uuid::new_v4();
    let now = Utc::now();

    let expired_session = TurnSession::from_persisted(PersistedTurnSessionData {
        id: corbusier::agent_backend::domain::TurnSessionId::new(),
        backend_id,
        conversation_id,
        runtime_session_id: RuntimeSessionId::new("expired-session-id")?,
        status: TurnSessionStatus::Active,
        ttl_seconds: 30,
        started_at: now - Duration::seconds(90),
        last_used_at: now - Duration::seconds(90),
        expires_at: now - Duration::seconds(1),
        ended_at: None,
        turn_count: 3,
    });
    context
        .session_repository
        .upsert_session(&context.ctx, &expired_session)
        .await?;

    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("rotated", Vec::new()))?;

    let response = context
        .service
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "next", Vec::new()),
            ),
        )
        .await?;

    assert!(response.rotated_session());
    assert_ne!(response.runtime_session_id(), "expired-session-id");

    let sessions = context.session_repository.all_sessions()?;
    let prior = sessions
        .iter()
        .find(|session| session.id() == expired_session.id())
        .ok_or_else(|| eyre::eyre!("missing persisted expired session"))?;
    assert_eq!(prior.status(), TurnSessionStatus::Expired);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn propagates_tool_router_failures(context: TestContext) -> Result<(), eyre::Report> {
    let backend_id = corbusier::agent_backend::domain::BackendId::from_uuid(
        register_backend(&context, "claude_code_sdk").await?,
    );

    context.runtime.queue_turn_result(TurnExecutionResult::new(
        "response",
        vec![ToolCallRequest::new("fail_tool", json!({"x": 1}))?],
    ))?;
    context.router.fail_tool("fail_tool", "simulated failure")?;

    let result = context
        .service
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(Uuid::new_v4(), "run", Vec::new()),
            ),
        )
        .await;

    assert!(matches!(
        result,
        Err(AgentTurnOrchestrationError::ToolRouting { .. })
    ));
    Ok(())
}
