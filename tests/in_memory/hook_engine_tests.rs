//! In-memory integration tests for hook engine execution.

use corbusier::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository,
};
use corbusier::hook_engine::domain::{
    HookAction, HookActionId, HookActionType, HookDefinition, HookId, HookTriggerContext,
    HookTriggerType,
};
use corbusier::hook_engine::ports::{HookEngine, HookExecutionLogRepository};
use corbusier::hook_engine::services::HookEngineService;
use mockable::DefaultClock;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_pre_commit_hook_persists_results() {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let service = HookEngineService::new(
        Arc::new(definition_repo.clone()),
        Arc::new(action_executor),
        Arc::new(execution_log.clone()),
        Arc::new(DefaultClock),
    );

    let hook_id = HookId::new("hook-pre-commit").expect("valid hook id");
    let action_id = HookActionId::new("action-pre-commit").expect("valid action id");
    let definition = HookDefinition::new(
        hook_id,
        "Pre-commit hook",
        HookTriggerType::PreCommit,
        vec![HookAction::new(action_id, HookActionType::QualityGate)],
    )
    .expect("definition should be valid");

    definition_repo.insert(definition).expect("insert succeeds");

    let context = HookTriggerContext::new(HookTriggerType::PreCommit, &DefaultClock);
    let trigger_context_id = context.id();
    let results = service.execute(context).await.expect("execution succeeds");

    assert_eq!(results.len(), 1);
    let stored = execution_log
        .find_by_trigger_context(trigger_context_id)
        .await
        .expect("lookup succeeds");
    assert_eq!(stored.len(), 1);
}
