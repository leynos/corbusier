//! Shared fixtures and helpers for hook engine service tests.

use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use crate::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository, InMemoryHookPolicyAuditRepository,
};
use crate::hook_engine::domain::{
    ActionStatus, HookAction, HookActionId, HookActionType, HookDefinition, HookExecutionScope,
    HookId, HookPriority, HookTriggerContext, HookTriggerType, TriggerContextId,
};
use crate::hook_engine::ports::HookPolicyAuditRepository;
use crate::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use mockable::{Clock, DefaultClock};
use rstest::fixture;
use serde_json::json;
use std::sync::Arc;

pub(crate) type InMemoryHookEngineService = HookEngineService<
    InMemoryHookDefinitionRepository,
    InMemoryHookActionExecutor,
    InMemoryHookExecutionLogRepository,
    InMemoryHookPolicyAuditRepository,
    DefaultClock,
>;

pub(crate) fn request_ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}

pub(crate) fn build_action(
    id: &str,
) -> Result<HookAction, crate::hook_engine::domain::HookDomainError> {
    let action_id = HookActionId::new(id)?;
    Ok(HookAction::new(action_id, HookActionType::QualityGate))
}

pub(crate) fn build_definition(
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

pub(crate) struct HookEngineFixture {
    pub(crate) definition_repo: InMemoryHookDefinitionRepository,
    pub(crate) action_executor: InMemoryHookActionExecutor,
    pub(crate) execution_log: InMemoryHookExecutionLogRepository,
    pub(crate) policy_audit: InMemoryHookPolicyAuditRepository,
    pub(crate) service: InMemoryHookEngineService,
}

#[fixture]
pub(crate) fn hook_engine_fixture() -> HookEngineFixture {
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

pub(crate) async fn setup_failing_post_deploy_hook(
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

pub(crate) async fn setup_invalid_post_deploy_policy_hook(
    ctx: &RequestContext,
    definition_repo: &InMemoryHookDefinitionRepository,
    action_executor: &InMemoryHookActionExecutor,
) {
    let action_id = HookActionId::new("invalid-policy-action")
        .expect("invalid policy action id should be valid");
    let hook_id =
        HookId::new("hook-invalid-policy").expect("invalid policy hook id should be valid");
    let definition = HookDefinition::new(
        hook_id,
        "Invalid policy hook",
        HookTriggerType::PostDeploy,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .expect("invalid policy hook definition should be valid for test")
    .with_priority(HookPriority::new(1));

    definition_repo
        .insert(ctx, definition)
        .await
        .expect("insert invalid policy definition should succeed");
    action_executor
        .set_output(action_id.as_str(), json!({"status": "succeeded"}))
        .expect("configuring invalid policy output should succeed");
}

pub(crate) async fn setup_deny_policy_hook(
    ctx: &RequestContext,
    definition_repo: &InMemoryHookDefinitionRepository,
    action_executor: &InMemoryHookActionExecutor,
    scope: HookExecutionScope,
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
        scope.with_metadata(json!({"tool_name": "read_file"})),
        DefaultClock.utc(),
    )
}

#[expect(
    clippy::struct_field_names,
    reason = "ID suffix provides clarity for distinct identifier types"
)]
pub(crate) struct PolicyAuditQueryKeys {
    pub(crate) task_id: TaskId,
    pub(crate) conversation_id: ConversationId,
    pub(crate) trigger_context_id: TriggerContextId,
}

pub(crate) async fn assert_policy_audit_indexed_once(
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
