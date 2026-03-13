//! Service tests for hook engine execution.

use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use crate::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository,
};
use crate::hook_engine::domain::{
    ActionStatus, HookAction, HookActionId, HookActionType, HookDefinition, HookExecutionStatus,
    HookId, HookPriority, HookTriggerContext, HookTriggerType,
};
use crate::hook_engine::ports::{HookEngine, HookExecutionLogRepository};
use crate::hook_engine::services::HookEngineService;
use mockable::DefaultClock;
use std::sync::Arc;

fn request_ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}

fn build_action(id: &str) -> Result<HookAction, crate::hook_engine::domain::HookDomainError> {
    let action_id = HookActionId::new(id)?;
    Ok(HookAction::new(action_id, HookActionType::QualityGate))
}

fn build_definition(
    id: &str,
    priority: u16,
) -> Result<HookDefinition, crate::hook_engine::domain::HookDomainError> {
    let hook_id = HookId::new(id)?;
    HookDefinition::new(
        hook_id,
        format!("Hook {id}"),
        HookTriggerType::PreCommit,
        vec![build_action(&format!("action-{id}"))?],
    )
    .map(|definition| definition.with_priority(HookPriority::new(priority)))
}

#[tokio::test(flavor = "multi_thread")]
async fn execute_orders_hooks_by_priority_then_id() {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let service = HookEngineService::new(
        Arc::new(definition_repo.clone()),
        Arc::new(action_executor),
        Arc::new(execution_log.clone()),
        Arc::new(DefaultClock),
    );
    let ctx = request_ctx();

    definition_repo
        .insert(
            &ctx,
            build_definition("hook-b", 10).expect("hook-b definition should be valid for test"),
        )
        .await
        .expect("insert hook-b definition should succeed");
    definition_repo
        .insert(
            &ctx,
            build_definition("hook-a", 5).expect("hook-a definition should be valid for test"),
        )
        .await
        .expect("insert hook-a definition should succeed");
    definition_repo
        .insert(
            &ctx,
            build_definition("hook-c", 5).expect("hook-c definition should be valid for test"),
        )
        .await
        .expect("insert hook-c definition should succeed");

    let context = HookTriggerContext::new(HookTriggerType::PreCommit, &DefaultClock);
    let results = service
        .execute(&ctx, context)
        .await
        .expect("pre-commit hook execution should succeed");

    let hook_ids: Vec<&str> = results
        .iter()
        .map(|result| result.hook_id().as_str())
        .collect();
    assert_eq!(hook_ids, vec!["hook-a", "hook-c", "hook-b"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn execute_persists_results_and_failure_status() {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let service = HookEngineService::new(
        Arc::new(definition_repo.clone()),
        Arc::new(action_executor.clone()),
        Arc::new(execution_log.clone()),
        Arc::new(DefaultClock),
    );
    let ctx = request_ctx();

    let action_id = HookActionId::new("failing-action").expect("failing action id should be valid");
    let hook_id = HookId::new("hook-fail").expect("failing hook id should be valid");
    let definition = HookDefinition::new(
        hook_id,
        "Failing hook",
        HookTriggerType::PostDeploy,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .expect("failing hook definition should be valid for test")
    .with_priority(HookPriority::new(1));

    definition_repo
        .insert(&ctx, definition)
        .await
        .expect("insert failing definition should succeed");
    action_executor
        .set_outcome(action_id.as_str(), ActionStatus::Failed)
        .expect("configuring failing action outcome should succeed");

    let context = HookTriggerContext::new(HookTriggerType::PostDeploy, &DefaultClock);
    let trigger_context_id = context.id();
    let results = service
        .execute(&ctx, context)
        .await
        .expect("post-deploy hook execution should succeed");

    let result = results.first().expect("expected one execution result");
    assert_eq!(result.status(), HookExecutionStatus::Failed);

    let stored = execution_log
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("querying stored execution results should succeed");
    assert_eq!(stored.len(), 1);
    let stored_result = stored
        .first()
        .expect("expected one stored execution result");
    assert_eq!(stored_result.status(), HookExecutionStatus::Failed);
}
