//! Execution-order and persistence tests for the hook engine service.

use super::common::{
    HookEngineFixture, build_definition, hook_engine_fixture, request_ctx,
    setup_failing_post_deploy_hook,
};
use crate::hook_engine::domain::{
    HookAction, HookActionId, HookActionType, HookDefinition, HookExecutionId, HookExecutionInput,
    HookExecutionStatus, HookId, HookPriority, HookTriggerContext, HookTriggerType,
};
use crate::hook_engine::ports::{
    HookEngine, HookExecutionLogError, HookExecutionLogRepository, HookPolicyAuditRepository,
};
use eyre::Result;
use mockable::DefaultClock;
use serde_json::json;

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

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn execute_persists_results_and_failure_status(
    hook_engine_fixture: HookEngineFixture,
) -> Result<()> {
    let HookEngineFixture {
        definition_repo,
        action_executor,
        execution_log,
        policy_audit,
        service,
    } = hook_engine_fixture;
    let ctx = request_ctx();

    setup_failing_post_deploy_hook(&ctx, &definition_repo, &action_executor).await?;

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
    Ok(())
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
