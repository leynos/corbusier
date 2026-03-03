//! Service tests for hook engine execution.

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

fn build_action(id: &str) -> HookAction {
    let action_id = HookActionId::new(id).expect("valid action id");
    HookAction::new(action_id, HookActionType::QualityGate)
}

fn build_definition(id: &str, priority: u16) -> HookDefinition {
    let hook_id = HookId::new(id).expect("valid hook id");
    HookDefinition::new(
        hook_id,
        format!("Hook {id}"),
        HookTriggerType::PreCommit,
        vec![build_action(&format!("action-{id}"))],
    )
    .expect("definition should be valid")
    .with_priority(HookPriority::new(priority))
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

    definition_repo
        .insert(build_definition("hook-b", 10))
        .expect("insert succeeds");
    definition_repo
        .insert(build_definition("hook-a", 5))
        .expect("insert succeeds");
    definition_repo
        .insert(build_definition("hook-c", 5))
        .expect("insert succeeds");

    let context = HookTriggerContext::new(HookTriggerType::PreCommit, &DefaultClock);
    let results = service.execute(context).await.expect("execution succeeds");

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

    let action_id = HookActionId::new("failing-action").expect("valid action id");
    let hook_id = HookId::new("hook-fail").expect("valid hook id");
    let definition = HookDefinition::new(
        hook_id,
        "Failing hook",
        HookTriggerType::PostDeploy,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .expect("definition should be valid")
    .with_priority(HookPriority::new(1));

    definition_repo.insert(definition).expect("insert succeeds");
    action_executor
        .set_outcome(action_id.as_str(), ActionStatus::Failed)
        .expect("status set");

    let context = HookTriggerContext::new(HookTriggerType::PostDeploy, &DefaultClock);
    let trigger_context_id = context.id();
    let results = service.execute(context).await.expect("execution succeeds");

    let result = results.first().expect("one result");
    assert_eq!(result.status(), HookExecutionStatus::Failed);

    let stored = execution_log
        .find_by_trigger_context(trigger_context_id)
        .await
        .expect("lookup succeeds");
    assert_eq!(stored.len(), 1);
}
