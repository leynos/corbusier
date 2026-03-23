//! In-memory integration tests for agent turn orchestration and sessions.

use std::sync::Arc;

use chrono::{Duration, Utc};
use corbusier::agent_backend::{
    domain::{
        BackendId, PersistedTurnSessionData, RuntimeSessionId, ToolCallRequest,
        TurnExecutionRequest, TurnExecutionResult, TurnSession, TurnSessionStatus,
    },
    ports::{
        SessionSlotArbitration, SessionSlotKey, SessionSlotReservation, TurnSessionRepository,
    },
    services::{
        AgentTurnOrchestrationError, BackendRegistryService, ExecuteAgentTurnRequest,
        RegisterBackendRequest,
    },
};
use corbusier::test_support::{InMemoryAgentTurnStack, build_in_memory_orchestrator};
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

type TestContext = InMemoryAgentTurnStack;

#[fixture]
fn context() -> TestContext {
    build_in_memory_orchestrator()
}

async fn register_backend(context: &TestContext, name: &str) -> Result<BackendId, eyre::Report> {
    let request = RegisterBackendRequest::new(name, name, "1.0.0", "test-provider")
        .with_capabilities(true, true);
    let registry_service =
        BackendRegistryService::new(context.backend_registry.clone(), Arc::new(DefaultClock));
    let backend = registry_service.register(&context.ctx, request).await?;
    Ok(backend.id())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn orchestrates_turn_and_reuses_session_before_expiry(
    context: TestContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;
    let conversation_id = Uuid::new_v4();

    context.runtime.queue_turn_result(TurnExecutionResult::new(
        "first-response",
        vec![ToolCallRequest::new("lookup", json!({"q": "roadmap"}))?],
    ))?;
    context
        .tool_router
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
async fn reservation_is_persisted_before_runtime_session_creation(
    context: TestContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;
    let conversation_id = Uuid::new_v4();

    let arbitration = context
        .session_repository
        .arbitrate_session_slot(
            &context.ctx,
            SessionSlotReservation::new(
                SessionSlotKey::new(backend_id, conversation_id),
                Utc::now(),
                Duration::minutes(5),
            ),
        )
        .await?;

    let SessionSlotArbitration::Reserved {
        reservation,
        prior_expired,
    } = arbitration
    else {
        return Err(eyre::eyre!("expected reserved arbitration result"));
    };
    assert!(prior_expired.is_none());
    assert!(context.runtime.created_session_ids()?.is_empty());

    let sessions = context.session_repository.all_sessions()?;
    assert_eq!(sessions.len(), 1);
    let persisted = sessions
        .first()
        .ok_or_else(|| eyre::eyre!("missing reservation row"))?;
    assert_eq!(persisted.id(), reservation.id());
    assert_eq!(persisted.status(), TurnSessionStatus::Reserved);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn timed_out_reservation_is_reclaimed(context: TestContext) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "claude_code_sdk").await?;
    let conversation_id = Uuid::new_v4();
    let first_now = Utc::now();

    let first = context
        .session_repository
        .arbitrate_session_slot(
            &context.ctx,
            SessionSlotReservation::new(
                SessionSlotKey::new(backend_id, conversation_id),
                first_now,
                Duration::seconds(1),
            ),
        )
        .await?;
    let SessionSlotArbitration::Reserved {
        reservation: initial_reservation,
        ..
    } = first
    else {
        return Err(eyre::eyre!("expected initial reservation"));
    };

    let second = context
        .session_repository
        .arbitrate_session_slot(
            &context.ctx,
            SessionSlotReservation::new(
                SessionSlotKey::new(backend_id, conversation_id),
                first_now + Duration::seconds(2),
                Duration::minutes(5),
            ),
        )
        .await?;
    let SessionSlotArbitration::Reserved {
        reservation,
        prior_expired,
    } = second
    else {
        return Err(eyre::eyre!("expected reclaimed reservation"));
    };

    assert!(prior_expired.is_none());
    assert_ne!(reservation.id(), initial_reservation.id());

    let sessions = context.session_repository.all_sessions()?;
    let expired = sessions
        .iter()
        .find(|session| session.id() == initial_reservation.id())
        .ok_or_else(|| eyre::eyre!("missing expired reservation"))?;
    assert_eq!(expired.status(), TurnSessionStatus::Expired);
    let reserved_count = sessions
        .iter()
        .filter(|session| session.status() == TurnSessionStatus::Reserved)
        .count();
    assert_eq!(reserved_count, 1);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn rotates_expired_session_and_marks_prior_session_expired(
    context: TestContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "codex_cli").await?;
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
    let backend_id = register_backend(&context, "claude_code_sdk").await?;

    context.runtime.queue_turn_result(TurnExecutionResult::new(
        "response",
        vec![ToolCallRequest::new("fail_tool", json!({"x": 1}))?],
    ))?;
    context
        .tool_router
        .fail_tool("fail_tool", "simulated failure")?;

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
