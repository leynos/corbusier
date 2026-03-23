//! Error propagation orchestration tests.

use super::common::{OrchestrationContext, context, register_backend};
use crate::agent_backend::{
    domain::{ToolCallRequest, TurnExecutionRequest, TurnExecutionResult, TurnSessionStatus},
    services::{AgentTurnOrchestrationError, ExecuteAgentTurnRequest},
};
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_turn_propagates_runtime_failure(
    context: OrchestrationContext,
) -> Result<(), eyre::Report> {
    let backend_id = register_backend(&context, "codex_cli").await?;
    context.runtime.fail_next_execute("backend unavailable")?;

    let result = context
        .service
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(Uuid::new_v4(), "Run turn", Vec::new()),
            ),
        )
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
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(Uuid::new_v4(), "Fail create session", Vec::new()),
            ),
        )
        .await;

    assert!(matches!(
        result,
        Err(AgentTurnOrchestrationError::Runtime(_))
    ));

    let sessions = context.session_repository.all_sessions()?;
    assert_eq!(sessions.len(), 1);
    let reservation = sessions
        .first()
        .ok_or_else(|| eyre::eyre!("expected expired reservation session"))?;
    assert_eq!(reservation.status(), TurnSessionStatus::Expired);
    assert_eq!(reservation.turn_count(), 0);
    assert!(context.runtime.created_session_ids()?.is_empty());
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
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(Uuid::new_v4(), "Trigger routing", Vec::new()),
            ),
        )
        .await;

    let expected_call_id =
        crate::agent_backend::domain::deterministic_tool_call_id(&failing_call, 0);
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
