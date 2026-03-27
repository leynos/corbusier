//! Policy audit projection and persistence tests for the hook engine service.

use super::common::{
    HookEngineFixture, PolicyAuditQueryKeys, assert_policy_audit_indexed_once, hook_engine_fixture,
    request_ctx, setup_deny_policy_hook, setup_failing_post_deploy_hook,
    setup_invalid_post_deploy_policy_hook,
};
use crate::context::RequestContext;
use crate::hook_engine::domain::{
    HookExecutionScope, HookExecutionStatus, HookTriggerContext, HookTriggerType,
};
use crate::hook_engine::ports::{
    HookEngine, HookEngineError, HookExecutionLogRepository, HookPolicyAuditError,
    HookPolicyAuditRepository,
};
use crate::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use async_trait::async_trait;
use mockable::DefaultClock;
use std::sync::Arc;

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
    let scope = HookExecutionScope::default()
        .with_task_id(task_id)
        .with_conversation_id(conversation_id);
    let context = setup_deny_policy_hook(&ctx, &definition_repo, &action_executor, scope).await;
    let trigger_context_id = context.id();
    service
        .execute(&ctx, context)
        .await
        .expect("policy hook execution should succeed");

    assert_policy_audit_indexed_once(
        &policy_audit,
        &ctx,
        PolicyAuditQueryKeys {
            task_id,
            conversation_id,
            trigger_context_id,
        },
    )
    .await;
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn policy_audit_projection_error_surfaces_without_persisting_events(
    hook_engine_fixture: HookEngineFixture,
) {
    let HookEngineFixture {
        definition_repo,
        action_executor,
        execution_log,
        policy_audit,
        service,
    } = hook_engine_fixture;
    let ctx = request_ctx();

    setup_invalid_post_deploy_policy_hook(&ctx, &definition_repo, &action_executor).await;

    let context = HookTriggerContext::new(HookTriggerType::PostDeploy, &DefaultClock);
    let trigger_context_id = context.id();
    let error = service
        .execute(&ctx, context)
        .await
        .expect_err("invalid policy output should fail audit projection");

    assert!(matches!(error, HookEngineError::PolicyAuditProjection(_)));

    let audit_events = policy_audit
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("querying policy audit events should succeed");
    assert!(
        audit_events.is_empty(),
        "no audit events should be stored when projection fails"
    );

    let executions = execution_log
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("querying stored execution results should succeed");
    assert_eq!(executions.len(), 1, "expected a single stored execution");
    assert_eq!(
        executions
            .first()
            .expect("expected stored execution result")
            .status(),
        HookExecutionStatus::Succeeded
    );
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn policy_audit_persistence_error_surfaces_without_persisting_events(
    hook_engine_fixture: HookEngineFixture,
) {
    let HookEngineFixture {
        definition_repo,
        action_executor,
        execution_log,
        policy_audit,
        ..
    } = hook_engine_fixture;
    let ctx = request_ctx();

    setup_failing_post_deploy_hook(&ctx, &definition_repo, &action_executor).await;

    let failing_policy_audit = FailingPolicyAuditRepository::new(policy_audit.clone());
    let service = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(definition_repo.clone()),
        action_executor: Arc::new(action_executor.clone()),
        execution_log: Arc::new(execution_log.clone()),
        policy_audit_repository: Arc::new(failing_policy_audit),
        clock: Arc::new(DefaultClock),
    });

    let context = HookTriggerContext::new(HookTriggerType::PostDeploy, &DefaultClock);
    let trigger_context_id = context.id();
    let error = service
        .execute(&ctx, context)
        .await
        .expect_err("policy audit persistence should fail");

    assert!(matches!(error, HookEngineError::PolicyAudit(_)));

    let audit_events = policy_audit
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("querying policy audit events should succeed");
    assert!(
        audit_events.is_empty(),
        "no audit events should be stored when persistence fails"
    );

    let executions = execution_log
        .find_by_trigger_context(&ctx, trigger_context_id)
        .await
        .expect("querying stored execution results should succeed");
    assert_eq!(executions.len(), 1, "expected a single stored execution");
}

#[derive(Clone, Debug)]
struct FailingPolicyAuditRepository<R> {
    inner: R,
}

impl<R> FailingPolicyAuditRepository<R> {
    fn new(inner: R) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<R> HookPolicyAuditRepository for FailingPolicyAuditRepository<R>
where
    R: HookPolicyAuditRepository + Send + Sync,
{
    async fn store(
        &self,
        _ctx: &RequestContext,
        _event: &crate::hook_engine::domain::PolicyAuditEvent,
    ) -> Result<(), HookPolicyAuditError> {
        Err(HookPolicyAuditError::persistence_failed(
            std::io::Error::other("failing test repository"),
        ))
    }

    async fn find_by_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> Result<Vec<crate::hook_engine::domain::PolicyAuditEvent>, HookPolicyAuditError> {
        self.inner.find_by_task(ctx, task_id).await
    }

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> Result<Vec<crate::hook_engine::domain::PolicyAuditEvent>, HookPolicyAuditError> {
        self.inner.find_by_conversation(ctx, conversation_id).await
    }

    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: crate::hook_engine::domain::TriggerContextId,
    ) -> Result<Vec<crate::hook_engine::domain::PolicyAuditEvent>, HookPolicyAuditError> {
        self.inner
            .find_by_trigger_context(ctx, trigger_context_id)
            .await
    }
}
