//! Session lifecycle and serialization orchestration tests.

use super::common::{OrchestrationContext, context, register_backend};
use crate::agent_backend::{
    domain::{
        PersistedTurnSessionData, RuntimeSessionId, TurnExecutionRequest, TurnExecutionResult,
        TurnSession, TurnSessionCreateParams, TurnSessionId, TurnSessionStatus,
    },
    ports::TurnSessionRepository,
    services::ExecuteAgentTurnRequest,
};
use chrono::{Duration, Utc};
use rstest::rstest;
use uuid::Uuid;

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
        .upsert_session(&context.ctx, &active_session)
        .await?;
    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("reused", Vec::new()))?;

    let reused_response = context
        .service
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(active_conversation, "Reuse", Vec::new()),
            ),
        )
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
    // Note: `SessionExecutionLocks` serializes concurrent calls within a single service
    // instance. Distributed concurrency across multiple service instances requires
    // DB-level locking (see migration concurrency discussion in PR `#36`).
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
        context.service.execute_turn(&context.ctx, first_request),
        context.service.execute_turn(&context.ctx, second_request)
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
    let active_sessions = sessions
        .iter()
        .filter(|session| session.backend_id() == backend_id)
        .filter(|session| session.conversation_id() == conversation_id)
        .filter(|session| session.status() == TurnSessionStatus::Active)
        .collect::<Vec<_>>();
    assert_eq!(
        active_sessions.len(),
        1,
        "expected exactly one active session for backend {backend_id:?} conversation {conversation_id}"
    );
    let active_session = active_sessions
        .first()
        .ok_or_else(|| eyre::eyre!("expected exactly one active session after filtering"))?;
    assert_eq!(active_session.turn_count(), 2);
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
        id: TurnSessionId::new(),
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
        .upsert_session(&context.ctx, &expired_session)
        .await?;
    context
        .runtime
        .queue_turn_result(TurnExecutionResult::new("rotated", Vec::new()))?;

    let rotated_response = context
        .service
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(expired_conversation, "Rotate", Vec::new()),
            ),
        )
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
