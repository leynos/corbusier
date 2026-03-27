//! In-memory integration tests for hook engine execution.

use corbusier::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository, InMemoryHookPolicyAuditRepository,
};
use corbusier::hook_engine::domain::{
    HookAction, HookActionId, HookActionType, HookDefinition, HookExecutionScope, HookId,
    HookTriggerContext, HookTriggerType,
};
use corbusier::hook_engine::ports::{
    HookEngine, HookExecutionLogRepository, HookPolicyAuditRepository,
};
use corbusier::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use corbusier::test_support::other_tenant_ctx;
use corbusier::test_support::test_request_ctx;
use corbusier::{message::domain::ConversationId, task::domain::TaskId};
use eyre::Result;
use mockable::{Clock, DefaultClock};
use serde_json::json;
use std::sync::Arc;

struct PolicyAuditFixture {
    service: HookEngineService<
        InMemoryHookDefinitionRepository,
        InMemoryHookActionExecutor,
        InMemoryHookExecutionLogRepository,
        InMemoryHookPolicyAuditRepository,
        DefaultClock,
    >,
    definition_repo: InMemoryHookDefinitionRepository,
    action_executor: InMemoryHookActionExecutor,
    policy_audit: InMemoryHookPolicyAuditRepository,
}

fn policy_audit_fixture() -> PolicyAuditFixture {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let policy_audit = InMemoryHookPolicyAuditRepository::new();
    let service = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(definition_repo.clone()),
        action_executor: Arc::new(action_executor.clone()),
        execution_log: Arc::new(execution_log),
        policy_audit_repository: Arc::new(policy_audit.clone()),
        clock: Arc::new(DefaultClock),
    });
    PolicyAuditFixture {
        service,
        definition_repo,
        action_executor,
        policy_audit,
    }
}

fn policy_audit_definition(action_id: &HookActionId) -> Result<HookDefinition> {
    let hook_id = HookId::new("hook-policy-audit")?;
    HookDefinition::new(
        hook_id,
        "Policy audit hook",
        HookTriggerType::PreToolUse,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .map_err(Into::into)
}

struct QueryExpectation<'a> {
    policy_audit: &'a InMemoryHookPolicyAuditRepository,
    ctx: &'a corbusier::context::RequestContext,
    task_id: TaskId,
    conversation_id: ConversationId,
    trigger_context_id: corbusier::hook_engine::domain::TriggerContextId,
}

async fn assert_policy_audit_queries(expectation: QueryExpectation<'_>) -> Result<()> {
    assert_eq!(
        expectation
            .policy_audit
            .find_by_task(expectation.ctx, expectation.task_id)
            .await?
            .len(),
        1
    );
    assert_eq!(
        expectation
            .policy_audit
            .find_by_conversation(expectation.ctx, expectation.conversation_id)
            .await?
            .len(),
        1
    );
    assert_eq!(
        expectation
            .policy_audit
            .find_by_trigger_context(expectation.ctx, expectation.trigger_context_id)
            .await?
            .len(),
        1
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_pre_commit_hook_persists_results() {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let policy_audit = InMemoryHookPolicyAuditRepository::new();
    let service = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(definition_repo.clone()),
        action_executor: Arc::new(action_executor),
        execution_log: Arc::new(execution_log.clone()),
        policy_audit_repository: Arc::new(policy_audit.clone()),
        clock: Arc::new(DefaultClock),
    });

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

    let expected = results.first().expect("expected execution result");
    let persisted = stored.first().expect("expected stored result");
    assert_eq!(persisted.execution_id(), expected.execution_id());
    assert_eq!(persisted.status(), expected.status());
}

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_hook_history_is_tenant_isolated() {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let policy_audit = InMemoryHookPolicyAuditRepository::new();
    let service = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(definition_repo.clone()),
        action_executor: Arc::new(action_executor),
        execution_log: Arc::new(execution_log.clone()),
        policy_audit_repository: Arc::new(policy_audit.clone()),
        clock: Arc::new(DefaultClock),
    });

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

#[tokio::test(flavor = "multi_thread")]
async fn in_memory_policy_audit_is_queryable_by_task_and_conversation() -> Result<()> {
    let PolicyAuditFixture {
        service,
        definition_repo,
        action_executor,
        policy_audit,
    } = policy_audit_fixture();

    let ctx = test_request_ctx();
    let task_id = TaskId::new();
    let conversation_id = ConversationId::new();
    let action_id = HookActionId::new("action-policy-audit").expect("valid action id");
    let definition = policy_audit_definition(&action_id)?;

    definition_repo
        .insert(&ctx, definition)
        .await
        .expect("insert succeeds");
    action_executor
        .set_output(
            action_id.as_str(),
            json!({
                "decision": "deny",
                "violation": {
                    "code": "tool.blocked",
                    "reason": "tool use is forbidden",
                }
            }),
        )
        .expect("configure policy output succeeds");

    let context = HookTriggerContext::new_with_timestamp(
        HookTriggerType::PreToolUse,
        HookExecutionScope::default()
            .with_task_id(task_id)
            .with_conversation_id(conversation_id)
            .with_metadata(json!({"tool_name": "read_file"})),
        DefaultClock.utc(),
    );
    let trigger_context_id = context.id();
    service
        .execute(&ctx, context)
        .await
        .expect("execution succeeds");
    assert_policy_audit_queries(QueryExpectation {
        policy_audit: &policy_audit,
        ctx: &ctx,
        task_id,
        conversation_id,
        trigger_context_id,
    })
    .await?;
    Ok(())
}
