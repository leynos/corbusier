//! Service tests for hook engine execution.

use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use crate::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository, InMemoryHookPolicyAuditRepository,
};
use crate::hook_engine::domain::{
    ActionStatus, HookAction, HookActionId, HookActionType, HookDefinition, HookExecutionId,
    HookExecutionInput, HookExecutionScope, HookExecutionStatus, HookId, HookPriority,
    HookTriggerContext, HookTriggerType, TriggerContextId,
};
use crate::hook_engine::ports::{
    HookEngine, HookExecutionLogError, HookExecutionLogRepository, HookPolicyAuditRepository,
};
use crate::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use mockable::{Clock, DefaultClock};
use rstest::fixture;
use serde_json::json;
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

struct HookEngineFixture {
    definition_repo: InMemoryHookDefinitionRepository,
    action_executor: InMemoryHookActionExecutor,
    execution_log: InMemoryHookExecutionLogRepository,
    policy_audit: InMemoryHookPolicyAuditRepository,
    service: HookEngineService<
        InMemoryHookDefinitionRepository,
        InMemoryHookActionExecutor,
        InMemoryHookExecutionLogRepository,
        InMemoryHookPolicyAuditRepository,
        DefaultClock,
    >,
}

#[fixture]
fn hook_engine_fixture() -> HookEngineFixture {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let policy_audit = InMemoryHookPolicyAuditRepository::new();
    let service = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(definition_repo.clone()),
        action_executor: Arc::new(action_executor.clone()),
        execution_log: Arc::new(execution_log.clone()),
        policy_audit_repository: Arc::new(policy_audit.clone()),
        clock: Arc::new(DefaultClock),
    });
    HookEngineFixture {
        definition_repo,
        action_executor,
        execution_log,
        policy_audit,
        service,
    }
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_orders_hooks_by_priority_then_id(hook_engine_fixture: HookEngineFixture) {
    let HookEngineFixture {
        definition_repo,
        service,
        ..
    } = hook_engine_fixture;
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

async fn setup_failing_post_deploy_hook(
    ctx: &RequestContext,
    definition_repo: &InMemoryHookDefinitionRepository,
    action_executor: &InMemoryHookActionExecutor,
) {
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
        .insert(ctx, definition)
        .await
        .expect("insert failing definition should succeed");
    action_executor
        .set_outcome(action_id.as_str(), ActionStatus::Failed)
        .expect("configuring failing action outcome should succeed");
    action_executor
        .set_output(
            action_id.as_str(),
            json!({
                "decision": "deny",
                "reason": "policy blocked deployment",
            }),
        )
        .expect("configuring failing action output should succeed");
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_persists_results_and_failure_status(hook_engine_fixture: HookEngineFixture) {
    let HookEngineFixture {
        definition_repo,
        action_executor,
        execution_log,
        policy_audit,
        service,
    } = hook_engine_fixture;
    let ctx = request_ctx();

    setup_failing_post_deploy_hook(&ctx, &definition_repo, &action_executor).await;

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

    let audit_events = policy_audit
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("querying policy audit events should succeed");
    assert_eq!(audit_events.len(), 1);
    assert_eq!(
        audit_events
            .first()
            .expect("expected policy audit event")
            .violation()
            .expect("deny event should record violation")
            .reason(),
        "policy blocked deployment"
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "The extracted fixture helper needs task and conversation correlation inputs."
)]
async fn setup_deny_policy_hook(
    ctx: &RequestContext,
    definition_repo: &InMemoryHookDefinitionRepository,
    action_executor: &InMemoryHookActionExecutor,
    task_id: TaskId,
    conversation_id: ConversationId,
) -> HookTriggerContext {
    let action_id = HookActionId::new("policy-action").expect("policy action id should be valid");
    let hook_id = HookId::new("hook-policy-scope").expect("hook id should be valid");
    let definition = HookDefinition::new(
        hook_id,
        "Policy hook",
        HookTriggerType::PreToolUse,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .expect("policy definition should be valid");

    definition_repo
        .insert(ctx, definition)
        .await
        .expect("insert policy definition should succeed");
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
        .expect("configuring policy output should succeed");

    HookTriggerContext::new_with_timestamp(
        HookTriggerType::PreToolUse,
        HookExecutionScope::default()
            .with_task_id(task_id)
            .with_conversation_id(conversation_id)
            .with_metadata(json!({"tool_name": "read_file"})),
        DefaultClock.utc(),
    )
}

#[expect(
    clippy::struct_field_names,
    reason = "ID suffix provides clarity for distinct identifier types"
)]
struct PolicyAuditQueryKeys {
    task_id: TaskId,
    conversation_id: ConversationId,
    trigger_context_id: TriggerContextId,
}

async fn assert_policy_audit_indexed_once(
    policy_audit: &InMemoryHookPolicyAuditRepository,
    ctx: &RequestContext,
    keys: PolicyAuditQueryKeys,
) {
    let by_task = policy_audit
        .find_by_task(ctx, keys.task_id)
        .await
        .expect("querying policy events by task should succeed");
    assert_eq!(
        by_task.len(),
        1,
        "expected exactly one policy event indexed by task"
    );
    let by_conversation = policy_audit
        .find_by_conversation(ctx, keys.conversation_id)
        .await
        .expect("querying policy events by conversation should succeed");
    assert_eq!(
        by_conversation.len(),
        1,
        "expected exactly one policy event indexed by conversation"
    );
    let by_trigger = policy_audit
        .find_by_trigger_context(ctx, keys.trigger_context_id)
        .await
        .expect("querying policy events by trigger should succeed");
    assert_eq!(
        by_trigger.len(),
        1,
        "expected exactly one policy event indexed by trigger context"
    );
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_reuses_existing_execution_without_replaying_actions(
    hook_engine_fixture: HookEngineFixture,
) {
    let HookEngineFixture {
        definition_repo,
        execution_log,
        policy_audit,
        action_executor,
        service,
        ..
    } = hook_engine_fixture;
    let ctx = request_ctx();
    let action_id = HookActionId::new("action-hook-retry").expect("valid action id");

    definition_repo
        .insert(
            &ctx,
            HookDefinition::new(
                HookId::new("hook-retry").expect("valid hook id"),
                "Retry hook",
                HookTriggerType::PreCommit,
                vec![HookAction::new(
                    action_id.clone(),
                    HookActionType::PolicyCheck,
                )],
            )
            .expect("retry definition should be valid for test")
            .with_priority(HookPriority::new(1)),
        )
        .await
        .expect("insert retry definition should succeed");
    action_executor
        .set_output(action_id.as_str(), json!({"decision": "allow"}))
        .expect("configuring retry action output should succeed");

    let context = HookTriggerContext::new(HookTriggerType::PreCommit, &DefaultClock);
    let trigger_context_id = context.id();
    let first_results = service
        .execute(&ctx, context.clone())
        .await
        .expect("initial hook execution should succeed");
    let second_results = service
        .execute(&ctx, context)
        .await
        .expect("duplicate hook execution should reuse the stored result");

    let first_result = first_results
        .first()
        .expect("expected one execution result on initial run");
    let second_result = second_results
        .first()
        .expect("expected one execution result on duplicate run");
    assert_eq!(first_result.execution_id(), second_result.execution_id());

    let stored = execution_log
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("querying stored retry execution should succeed");
    assert_eq!(stored.len(), 1);
    let audit_events = policy_audit
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("querying retry audit events should succeed");
    assert_eq!(audit_events.len(), 1);
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_projects_policy_audit_by_task_and_conversation(
    hook_engine_fixture: HookEngineFixture,
) {
    let HookEngineFixture {
        definition_repo,
        action_executor,
        policy_audit,
        service,
        ..
    } = hook_engine_fixture;
    let ctx = request_ctx();
    let task_id = TaskId::new();
    let conversation_id = ConversationId::new();
    let context = setup_deny_policy_hook(
        &ctx,
        &definition_repo,
        &action_executor,
        task_id,
        conversation_id,
    )
    .await;
    let trigger_context_id = context.id();
    service
        .execute(&ctx, context)
        .await
        .expect("policy hook execution should succeed");

    assert_policy_audit_indexed_once(
        &policy_audit,
        &ctx,
        PolicyAuditQueryKeys { task_id, conversation_id, trigger_context_id },
    )
    .await;
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn store_rejects_duplicate_trigger_context_and_hook(hook_engine_fixture: HookEngineFixture) {
    let HookEngineFixture { execution_log, .. } = hook_engine_fixture;
    let ctx = request_ctx();
    let hook_id = HookId::new("hook-duplicate-store").expect("hook id should be valid");
    let trigger_context = HookTriggerContext::new(HookTriggerType::PreCommit, &DefaultClock);
    let executed_at = trigger_context.occurred_at();
    let execution = crate::hook_engine::domain::HookExecutionResult::new(HookExecutionInput {
        execution_id: HookExecutionId::new(),
        hook_id: hook_id.clone(),
        trigger_context_id: trigger_context.id(),
        trigger_type: HookTriggerType::PreCommit,
        predicate_data: serde_json::Value::Null,
        action_results: Vec::new(),
        executed_at,
    });
    execution_log
        .store(&ctx, &execution)
        .await
        .expect("initial store should succeed");

    let duplicate = crate::hook_engine::domain::HookExecutionResult::new(HookExecutionInput {
        execution_id: HookExecutionId::new(),
        hook_id,
        trigger_context_id: trigger_context.id(),
        trigger_type: HookTriggerType::PreCommit,
        predicate_data: serde_json::Value::Null,
        action_results: Vec::new(),
        executed_at,
    });
    let error = execution_log
        .store(&ctx, &duplicate)
        .await
        .expect_err("duplicate trigger-context and hook should be rejected");

    assert!(matches!(
        error,
        HookExecutionLogError::PersistenceFailed { .. }
    ));
}
