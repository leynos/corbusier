//! Unit tests for agent turn orchestration service behaviour.

use std::sync::Arc;

use crate::agent_backend::{
    adapters::memory::{
        InMemoryAgentRuntime, InMemoryBackendRegistry, InMemoryToolRouter,
        InMemoryTurnSessionRepository,
    },
    domain::{
        AgentBackendRegistration, AgentCapabilities, BackendId, BackendInfo, BackendName,
        PersistedTurnSessionData, RuntimeSessionId, ToolCallAuditStatus, ToolCallRequest,
        TurnExecutionRequest, TurnExecutionResult, TurnSession, TurnSessionCreateParams,
        TurnSessionStatus, deterministic_tool_call_id,
    },
    ports::{BackendRegistryRepository, TurnSessionRepository},
    services::{
        AgentTurnOrchestrationError, AgentTurnOrchestratorConfig, AgentTurnOrchestratorPorts,
        AgentTurnOrchestratorService, ExecuteAgentTurnRequest,
    },
};
use chrono::{Duration, Utc};
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

struct OrchestrationContext {
    backend_registry: Arc<InMemoryBackendRegistry>,
    session_repository: Arc<InMemoryTurnSessionRepository>,
    runtime: Arc<InMemoryAgentRuntime>,
    tool_router: Arc<InMemoryToolRouter>,
    service: TestOrchestrator,
    clock: Arc<DefaultClock>,
}

#[fixture]
fn context() -> OrchestrationContext {
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
    }
}

fn create_backend_registration(
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

async fn register_backend(
    context: &OrchestrationContext,
    name: &str,
) -> Result<BackendId, eyre::Report> {
    let registration = create_backend_registration(name, context.clock.as_ref())?;
    let backend_id = registration.id();
    context.backend_registry.register(&registration).await?;
    Ok(backend_id)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_routes_tool_calls_and_returns_audits(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;
    let conversation_id = Uuid::new_v4();

    let tool_calls = vec![
        ToolCallRequest::new("search_docs", json!({"query": "roadmap"}))?,
        ToolCallRequest::new("summarize", json!({"format": "short"}))?,
    ];
    context.runtime.queue_turn_result(TurnExecutionResult::new(
        "assistant response",
        tool_calls.clone(),
    ))?;
    context
        .tool_router
        .set_tool_response("search_docs", json!({"matches": 4}))?;
    context
        .tool_router
        .set_tool_response("summarize", json!({"summary": "done"}))?;

    let turn = TurnExecutionRequest::new(conversation_id, "Process this", Vec::new());
    let response = context
        .service
        .execute_turn(ExecuteAgentTurnRequest::new(backend_id, turn))
        .await?;

    assert_eq!(response.assistant_response(), "assistant response");
    assert_eq!(response.tool_results().len(), 2);
    assert_eq!(response.tool_call_audits().len(), 2);
    assert!(!response.reused_session());
    assert!(!response.rotated_session());
    assert!(
        response
            .tool_call_audits()
            .iter()
            .all(|audit| audit.status() == ToolCallAuditStatus::Succeeded)
    );

    let routed_call_ids = context.tool_router.routed_call_ids()?;
    let result_call_ids: Vec<&str> = response
        .tool_results()
        .iter()
        .map(crate::agent_backend::domain::ToolCallResult::call_id)
        .collect();
    let routed_call_refs: Vec<&str> = routed_call_ids.iter().map(String::as_str).collect();
    assert_eq!(routed_call_refs, result_call_ids);

    let created_sessions = context.runtime.created_session_ids()?;
    assert_eq!(created_sessions.len(), 1);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_propagates_runtime_failure(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "codex_cli").await?;
    context.runtime.fail_next_execute("backend unavailable")?;

    let result = context
        .service
        .execute_turn(ExecuteAgentTurnRequest::new(
            backend_id,
            TurnExecutionRequest::new(Uuid::new_v4(), "Run turn", Vec::new()),
        ))
        .await;

    assert!(matches!(
        result,
        Err(AgentTurnOrchestrationError::Runtime(_))
    ));

    let sessions = context.session_repository.all_sessions()?;
    assert_eq!(sessions.len(), 1);
    let first_session = sessions
        .first()
        .ok_or_else(|| eyre::eyre!("expected one persisted session"))?;
    assert_eq!(first_session.turn_count(), 0);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_propagates_session_creation_failure(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;
    context.runtime.fail_session_creation_for(backend_id)?;

    let result = context
        .service
        .execute_turn(ExecuteAgentTurnRequest::new(
            backend_id,
            TurnExecutionRequest::new(Uuid::new_v4(), "Fail create session", Vec::new()),
        ))
        .await;

    assert!(matches!(
        result,
        Err(AgentTurnOrchestrationError::Runtime(_))
    ));

    let sessions = context.session_repository.all_sessions()?;
    assert_eq!(sessions.len(), 0);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_propagates_tool_routing_failure(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;

    let failing_call = ToolCallRequest::new("failing_tool", json!({"arg": 1}))?;
    context.runtime.queue_turn_result(TurnExecutionResult::new(
        "assistant response",
        vec![failing_call.clone()],
    ))?;
    context
        .tool_router
        .fail_tool("failing_tool", "simulated router failure")?;

    let result = context
        .service
        .execute_turn(ExecuteAgentTurnRequest::new(
            backend_id,
            TurnExecutionRequest::new(Uuid::new_v4(), "Trigger routing", Vec::new()),
        ))
        .await;

    let expected_call_id = deterministic_tool_call_id(&failing_call, 0);
    match result {
        Err(AgentTurnOrchestrationError::ToolRouting {
            call_id, tool_name, ..
        }) => {
            assert_eq!(call_id, expected_call_id);
            assert_eq!(tool_name, "failing_tool");
        }
        other => {
            return Err(eyre::eyre!("expected tool routing error, got {other:?}"));
        }
    }

    let sessions = context.session_repository.all_sessions()?;
    assert_eq!(sessions.len(), 1);
    let first_session = sessions
        .first()
        .ok_or_else(|| eyre::eyre!("expected one persisted session"))?;
    assert_eq!(first_session.turn_count(), 0);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_reuses_active_session(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;

    let now = Utc::now();

    let active_conversation = Uuid::new_v4();
    let active_session = TurnSession::new(TurnSessionCreateParams {
        backend_id,
        conversation_id: active_conversation,
        runtime_session_id: RuntimeSessionId::new("existing-runtime-session")?,
        ttl: Duration::seconds(300),
        now,
    })?;
    context
        .session_repository
        .upsert_session(&active_session)
        .await?;
    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("reused", Vec::new()))?;

    let reused_response = context
        .service
        .execute_turn(ExecuteAgentTurnRequest::new(
            backend_id,
            TurnExecutionRequest::new(active_conversation, "Reuse", Vec::new()),
        ))
        .await?;

    assert!(reused_response.reused_session());
    assert!(!reused_response.rotated_session());
    assert_eq!(
        reused_response.runtime_session_id(),
        "existing-runtime-session"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_serializes_concurrent_calls_for_same_session_key(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;
    let conversation_id = Uuid::new_v4();

    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("first", Vec::new()))?;
    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("second", Vec::new()))?;

    let first_request = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "1", Vec::new()),
    );
    let second_request = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "2", Vec::new()),
    );

    let (first_result, second_result) = tokio::join!(
        context.service.execute_turn(first_request),
        context.service.execute_turn(second_request)
    );

    let first_response = first_result?;
    let second_response = second_result?;
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

    let created_sessions = context.runtime.created_session_ids()?;
    assert_eq!(created_sessions.len(), 1);

    let sessions = context.session_repository.all_sessions()?;
    let active = sessions
        .iter()
        .find(|session| session.status() == TurnSessionStatus::Active)
        .ok_or_else(|| eyre::eyre!("expected an active session"))?;
    assert_eq!(active.turn_count(), 2);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_rotates_expired_session(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;
    let now = Utc::now();
    let expired_conversation = Uuid::new_v4();
    let expired_session = TurnSession::from_persisted(PersistedTurnSessionData {
        id: crate::agent_backend::domain::TurnSessionId::new(),
        backend_id,
        conversation_id: expired_conversation,
        runtime_session_id: RuntimeSessionId::new("expired-runtime-session")?,
        status: TurnSessionStatus::Active,
        ttl_seconds: 60,
        started_at: now - Duration::seconds(120),
        last_used_at: now - Duration::seconds(120),
        expires_at: now - Duration::seconds(1),
        ended_at: None,
        turn_count: 2,
    });
    context
        .session_repository
        .upsert_session(&expired_session)
        .await?;
    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("rotated", Vec::new()))?;

    let rotated_response = context
        .service
        .execute_turn(ExecuteAgentTurnRequest::new(
            backend_id,
            TurnExecutionRequest::new(expired_conversation, "Rotate", Vec::new()),
        ))
        .await?;

    assert!(!rotated_response.reused_session());
    assert!(rotated_response.rotated_session());
    assert_ne!(
        rotated_response.runtime_session_id(),
        "expired-runtime-session"
    );

    let sessions = context.session_repository.all_sessions()?;
    let expired = sessions
        .iter()
        .find(|session| session.id() == expired_session.id())
        .ok_or_else(|| eyre::eyre!("expired session missing"))?;
    assert_eq!(expired.status(), TurnSessionStatus::Expired);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_produces_deterministic_tool_call_order_for_identical_input(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "codex_cli").await?;
    let conversation_id = Uuid::new_v4();

    let tool_calls = vec![
        ToolCallRequest::new("tool_alpha", json!({"z": 1, "a": 2}))?,
        ToolCallRequest::new("tool_beta", json!({"nested": {"b": 1, "a": 2}}))?,
    ];

    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("first", tool_calls.clone()))?;
    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("second", tool_calls.clone()))?;

    let first_response = context
        .service
        .execute_turn(ExecuteAgentTurnRequest::new(
            backend_id,
            TurnExecutionRequest::new(conversation_id, "Run 1", Vec::new()),
        ))
        .await?;

    let second_response = context
        .service
        .execute_turn(ExecuteAgentTurnRequest::new(
            backend_id,
            TurnExecutionRequest::new(conversation_id, "Run 2", Vec::new()),
        ))
        .await?;

    let first_ids: Vec<&str> = first_response
        .tool_results()
        .iter()
        .map(crate::agent_backend::domain::ToolCallResult::call_id)
        .collect();
    let second_ids: Vec<&str> = second_response
        .tool_results()
        .iter()
        .map(crate::agent_backend::domain::ToolCallResult::call_id)
        .collect();
    assert_eq!(first_ids, second_ids);

    let routed_call_ids = context.tool_router.routed_call_ids()?;
    assert_eq!(routed_call_ids.len(), 4);
    let (first_half, second_half) = routed_call_ids.as_slice().split_at(2);
    assert_eq!(first_half, second_half);
    Ok(())
}
