//! Concurrency behaviour tests for orchestrated agent turns against `PostgreSQL`.

use chrono::Duration;
use corbusier::agent_backend::{
    domain::{TurnExecutionRequest, TurnExecutionResult, TurnSession, TurnSessionStatus},
    ports::TurnSessionRepositoryError,
    services::{
        AgentTurnOrchestrationError, AgentTurnOrchestratorConfig, AgentTurnOrchestratorPorts,
        AgentTurnOrchestratorService, ExecuteAgentTurnRequest,
    },
};
use rstest::rstest;
use uuid::Uuid;

use super::common::{OrchestrationContext, context, ensure_conversation_exists, register_backend};
use crate::postgres::helpers::BoxError;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_serializes_concurrent_calls_for_same_session_key(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = register_backend(&ctx, "claude_code_sdk").await?;
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

    let active_sessions = ctx
        .session_repository
        .all_sessions()
        .map_err(|err| Box::new(err) as BoxError)?
        .into_iter()
        .filter(|session| {
            session.backend_id() == backend_id
                && session.conversation_id() == conversation_id
                && session.status() == TurnSessionStatus::Active
        })
        .collect::<Vec<_>>();
    assert_eq!(
        active_sessions.len(),
        1,
        "expected exactly one active session row for the slot"
    );
    let active_session = active_sessions
        .into_iter()
        .next()
        .expect("Expected exactly one active session to exist");
    assert_eq!(active_session.turn_count(), 2);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn concurrent_execute_turn_creates_single_active_session(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = register_backend(&ctx, "concurrent_test_backend").await?;
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new("response-a", Vec::new()))
        .map_err(|err| Box::new(err) as BoxError)?;
    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new("response-b", Vec::new()))
        .map_err(|err| Box::new(err) as BoxError)?;

    let first_request = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "first turn", Vec::new()),
    );
    let second_request = ExecuteAgentTurnRequest::new(
        backend_id,
        TurnExecutionRequest::new(conversation_id, "second turn", Vec::new()),
    );
    let second_service = AgentTurnOrchestratorService::with_config(
        AgentTurnOrchestratorPorts {
            backend_registry: ctx.backend_registry.clone(),
            turn_sessions: ctx.session_repository.clone(),
            runtime: ctx.runtime.clone(),
            tool_router: ctx.router.clone(),
            clock: ctx.clock.clone(),
        },
        AgentTurnOrchestratorConfig::new(Duration::minutes(5))
            .map_err(|err| Box::new(err) as BoxError)?,
    );

    let (first_result, second_result) = tokio::join!(
        ctx.service.execute_turn(&ctx.ctx, first_request),
        second_service.execute_turn(&ctx.ctx, second_request)
    );
    let results = [first_result, second_result];
    let success_count = results.iter().filter(|result| result.is_ok()).count();
    assert_eq!(
        success_count, 1,
        "expected exactly one concurrent execute_turn call to win the reservation race"
    );
    let conflict_count = results
        .iter()
        .filter(|result| {
            matches!(
                result,
                Err(AgentTurnOrchestrationError::SessionRepository(
                    TurnSessionRepositoryError::ActiveSessionConflict { .. }
                ))
            )
        })
        .count();
    assert_eq!(
        conflict_count, 1,
        "expected exactly one concurrent execute_turn call to fail with an active-session conflict"
    );
    let winning_response = results
        .into_iter()
        .find_map(Result::ok)
        .ok_or_else(|| Box::new(std::io::Error::other("expected a successful turn")) as BoxError)?;

    let active_sessions = ctx
        .session_repository
        .all_sessions()
        .map_err(|err| Box::new(err) as BoxError)?
        .into_iter()
        .filter(|session| {
            session.backend_id() == backend_id
                && session.conversation_id() == conversation_id
                && session.status() == TurnSessionStatus::Active
        })
        .collect::<Vec<_>>();
    assert_eq!(
        active_sessions.len(),
        1,
        "expected exactly one active session row for the slot"
    );
    let active_session = active_sessions
        .into_iter()
        .next()
        .expect("Expected exactly one active session to exist");
    assert_eq!(
        active_session.id(),
        winning_response.session_id(),
        "Active session ID should match the successful turn session"
    );

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_serializes_concurrent_calls_with_different_backends_same_conversation(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_1 = register_backend(&ctx, "backend_alpha").await?;
    let backend_2 = register_backend(&ctx, "backend_beta").await?;
    let conversation_id = Uuid::new_v4();
    ensure_conversation_exists(&ctx, conversation_id).await?;

    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new("turn_1", Vec::new()))
        .map_err(|err| Box::new(err) as BoxError)?;
    ctx.runtime
        .queue_turn_result(TurnExecutionResult::new("turn_2", Vec::new()))
        .map_err(|err| Box::new(err) as BoxError)?;

    let request_1 = ExecuteAgentTurnRequest::new(
        backend_1,
        TurnExecutionRequest::new(conversation_id, "req_1", Vec::new()),
    );
    let request_2 = ExecuteAgentTurnRequest::new(
        backend_2,
        TurnExecutionRequest::new(conversation_id, "req_2", Vec::new()),
    );

    let (result_1, result_2) = tokio::join!(
        ctx.service.execute_turn(&ctx.ctx, request_1),
        ctx.service.execute_turn(&ctx.ctx, request_2)
    );

    let response_1 = result_1.map_err(|err| Box::new(err) as BoxError)?;
    let response_2 = result_2.map_err(|err| Box::new(err) as BoxError)?;

    assert_ne!(
        response_1.session_id(),
        response_2.session_id(),
        "different backends should create different sessions"
    );

    let created_sessions = ctx
        .runtime
        .created_session_ids()
        .map_err(|err| Box::new(err) as BoxError)?;
    assert_eq!(
        created_sessions.len(),
        2,
        "expected 2 runtime sessions (one per backend)"
    );

    let all_sessions = ctx
        .session_repository
        .all_sessions()
        .map_err(|err| Box::new(err) as BoxError)?;
    let conversation_sessions: Vec<_> = all_sessions
        .into_iter()
        .filter(|session| session.conversation_id() == conversation_id)
        .collect();
    assert_eq!(
        conversation_sessions.len(),
        2,
        "expected 2 sessions for the conversation (one per backend)"
    );

    let backend_ids: std::collections::HashSet<_> = conversation_sessions
        .iter()
        .map(TurnSession::backend_id)
        .collect();
    assert_eq!(
        backend_ids.len(),
        2,
        "expected sessions for 2 distinct backends"
    );
    assert!(
        backend_ids.contains(&backend_1),
        "expected session for backend_1"
    );
    assert!(
        backend_ids.contains(&backend_2),
        "expected session for backend_2"
    );

    Ok(())
}
