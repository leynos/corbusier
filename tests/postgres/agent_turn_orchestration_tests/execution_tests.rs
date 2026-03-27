//! End-to-end orchestration behaviour tests against `PostgreSQL`.

use chrono::{Duration, Utc};
use corbusier::agent_backend::{
    domain::{
        PersistedTurnSessionData, RuntimeSessionId, ToolCallRequest, TurnExecutionRequest,
        TurnExecutionResult, TurnSession, TurnSessionStatus,
    },
    ports::{SessionSlotKey, TurnSessionRepository},
    services::{AgentTurnOrchestrationError, ExecuteAgentTurnRequest},
};
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

use super::common::{OrchestrationContext, context, ensure_conversation_exists, register_backend};
use crate::postgres::helpers::BoxError;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_orchestrates_turn_and_reuses_session(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = register_backend(&ctx, "claude_code_sdk").await?;
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
    let backend_id = register_backend(&ctx, "codex_cli").await?;
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
        .upsert_session(&ctx.ctx, &expired_session)
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
        .find_active_session(&ctx.ctx, SessionSlotKey::new(backend_id, conversation_id))
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
async fn postgres_propagates_tool_routing_failure(
    #[future] context: Result<OrchestrationContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let backend_id = register_backend(&ctx, "claude_code_sdk").await?;
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
