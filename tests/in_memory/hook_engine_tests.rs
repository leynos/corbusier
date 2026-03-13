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
use corbusier::test_support::other_tenant_ctx;
use corbusier::test_support::test_request_ctx;
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

    let ctx = test_request_ctx();
    let hook_id = HookId::new("hook-pre-commit").expect("valid hook id");
    let action_id = HookActionId::new("action-pre-commit").expect("valid action id");
    let definition = HookDefinition::new(
        hook_id,
        "Pre-commit hook",
        HookTriggerType::PreCommit,
        vec![HookAction::new(action_id, HookActionType::QualityGate)],
    )
    .expect("definition should be valid");

    definition_repo
        .insert(&ctx, definition)
        .await
        .expect("insert succeeds");

    let context = HookTriggerContext::new(HookTriggerType::PreCommit, &DefaultClock);
    let trigger_context_id = context.id();
    let results = service
        .execute(&ctx, context)
        .await
        .expect("execution succeeds");

    assert_eq!(results.len(), 1);
    let stored = execution_log
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("lookup succeeds");
    assert_eq!(stored.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_hook_history_is_tenant_isolated() {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let service = HookEngineService::new(
        Arc::new(definition_repo.clone()),
        Arc::new(action_executor),
        Arc::new(execution_log.clone()),
        Arc::new(DefaultClock),
    );

    let tenant_a = test_request_ctx();
    let tenant_b = other_tenant_ctx(&tenant_a);
    let hook_id = HookId::new("hook-tenant-isolation").expect("valid hook id");
    let action_id = HookActionId::new("action-tenant-isolation").expect("valid action id");
    let definition = HookDefinition::new(
        hook_id,
        "Tenant-scoped hook",
        HookTriggerType::PreCommit,
        vec![HookAction::new(action_id, HookActionType::QualityGate)],
    )
    .expect("definition should be valid");

    definition_repo
        .insert(&tenant_a, definition)
        .await
        .expect("insert succeeds");

    let context = HookTriggerContext::new(HookTriggerType::PreCommit, &DefaultClock);
    let trigger_context_id = context.id();
    let results = service
        .execute(&tenant_a, context)
        .await
        .expect("execution succeeds");
    assert_eq!(results.len(), 1);
    let other_tenant_context = HookTriggerContext::new(HookTriggerType::PreCommit, &DefaultClock);
    let other_tenant_results = service
        .execute(&tenant_b, other_tenant_context)
        .await
        .expect("tenant B execution succeeds");
    assert!(other_tenant_results.is_empty());

    let own_tenant = execution_log
        .find_by_trigger_context(&tenant_a, trigger_context_id)
        .await
        .expect("tenant A lookup succeeds");
    assert_eq!(own_tenant.len(), 1);

    let other_tenant = execution_log
        .find_by_trigger_context(&tenant_b, trigger_context_id)
        .await
        .expect("tenant B lookup succeeds");
    assert!(other_tenant.is_empty());
}
