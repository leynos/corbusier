//! Determinism-focused orchestration tests.

use super::common::{OrchestrationContext, context, register_backend};
use crate::agent_backend::{
    domain::{ToolCallRequest, TurnExecutionRequest, TurnExecutionResult},
    services::ExecuteAgentTurnRequest,
};
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

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
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "Run 1", Vec::new()),
            ),
        )
        .await?;

    let second_response = context
        .service
        .execute_turn(
            &context.ctx,
            ExecuteAgentTurnRequest::new(
                backend_id,
                TurnExecutionRequest::new(conversation_id, "Run 2", Vec::new()),
            ),
        )
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
