//! Tool routing and audit-focused orchestration tests.

use super::common::{OrchestrationContext, context, register_backend};
use crate::agent_backend::{
    domain::{
        ToolCallAuditStatus, ToolCallRequest, ToolCallResult, TurnExecutionRequest,
        TurnExecutionResult,
    },
    services::ExecuteAgentTurnRequest,
};
use rstest::rstest;
use serde_json::json;
use uuid::Uuid;

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
        .execute_turn(&context.ctx, ExecuteAgentTurnRequest::new(backend_id, turn))
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
        .map(ToolCallResult::call_id)
        .collect();
    let routed_call_refs: Vec<&str> = routed_call_ids.iter().map(String::as_str).collect();
    assert_eq!(routed_call_refs, result_call_ids);

    let created_sessions = context.runtime.created_session_ids()?;
    assert_eq!(created_sessions.len(), 1);
    Ok(())
}
