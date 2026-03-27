//! `PostgreSQL` integration tests for hook engine execution logs.

use corbusier::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
};
use corbusier::hook_engine::adapters::postgres::{
    HookExecutionPgPool, PostgresHookExecutionLogRepository, PostgresHookPolicyAuditRepository,
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
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::{Clock, DefaultClock};
use rstest::fixture;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster,
};

type TestService = HookEngineService<
    InMemoryHookDefinitionRepository,
    InMemoryHookActionExecutor,
    PostgresHookExecutionLogRepository,
    PostgresHookPolicyAuditRepository,
    DefaultClock,
>;

struct HookEngineTestContext {
    service: TestService,
    definition_repo: InMemoryHookDefinitionRepository,
    execution_log: PostgresHookExecutionLogRepository,
    policy_audit: PostgresHookPolicyAuditRepository,
    _temp_db: TemporaryDatabase,
}

async fn setup_hook_engine_context(
    cluster: PostgresCluster,
) -> Result<HookEngineTestContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(&format!("hook_engine_{}", Uuid::new_v4()), TEMPLATE_DB)
        .await?;
    let url = db.url().to_owned();

    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool: HookExecutionPgPool = diesel::r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)?;
    let execution_log = PostgresHookExecutionLogRepository::new(pool.clone());
    let policy_audit = PostgresHookPolicyAuditRepository::new(pool);
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let service = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(definition_repo.clone()),
        action_executor: Arc::new(action_executor),
        execution_log: Arc::new(execution_log.clone()),
        policy_audit_repository: Arc::new(policy_audit.clone()),
        clock: Arc::new(DefaultClock),
    });

    Ok(HookEngineTestContext {
        service,
        definition_repo,
        execution_log,
        policy_audit,
        _temp_db: db,
    })
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<HookEngineTestContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_hook_engine_context(cluster).await
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_hook_execution_is_persisted(
    #[future] context: Result<HookEngineTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let request_ctx = test_request_ctx();

    let hook_id = HookId::new("hook-post-push").expect("valid hook id");
    let action_id = HookActionId::new("action-post-push").expect("valid action id");
    let definition = HookDefinition::new(
        hook_id,
        "Post-push hook",
        HookTriggerType::PostPush,
        vec![HookAction::new(action_id, HookActionType::Notification)],
    )
    .expect("definition should be valid");
    ctx.definition_repo
        .insert(&request_ctx, definition)
        .await
        .expect("insert succeeds");

    let trigger_context = HookTriggerContext::new(HookTriggerType::PostPush, &DefaultClock);
    let trigger_context_id = trigger_context.id();
    let results = ctx
        .service
        .execute(&request_ctx, trigger_context)
        .await
        .expect("execution succeeds");
    assert_eq!(results.len(), 1);

    let stored = ctx
        .execution_log
        .find_by_trigger_context(&request_ctx, trigger_context_id)
        .await
        .expect("lookup succeeds");
    assert_eq!(stored.len(), 1);

    let expected = results.first().expect("expected execution result");
    let persisted = stored.first().expect("expected stored result");
    assert_eq!(persisted.hook_id(), expected.hook_id());
    assert_eq!(persisted.status(), expected.status());
    assert_eq!(persisted.action_results(), expected.action_results());
    Ok(())
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_hook_execution_history_is_tenant_isolated(
    #[future] context: Result<HookEngineTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let tenant_a = test_request_ctx();
    let tenant_b = other_tenant_ctx(&tenant_a);

    let hook_id = HookId::new("hook-postgres-tenant-isolation").expect("valid hook id");
    let action_id = HookActionId::new("action-postgres-tenant-isolation").expect("valid action id");
    let definition = HookDefinition::new(
        hook_id,
        "Tenant-scoped postgres hook",
        HookTriggerType::PostPush,
        vec![HookAction::new(action_id, HookActionType::Notification)],
    )
    .expect("definition should be valid");
    ctx.definition_repo
        .insert(&tenant_a, definition)
        .await
        .expect("insert succeeds");

    let trigger_context = HookTriggerContext::new(HookTriggerType::PostPush, &DefaultClock);
    let trigger_context_id = trigger_context.id();
    let results = ctx
        .service
        .execute(&tenant_a, trigger_context)
        .await
        .expect("execution succeeds");
    assert_eq!(results.len(), 1);
    let other_tenant_context = HookTriggerContext::new(HookTriggerType::PostPush, &DefaultClock);
    let other_tenant_results = ctx
        .service
        .execute(&tenant_b, other_tenant_context)
        .await
        .expect("tenant B execution succeeds");
    assert!(other_tenant_results.is_empty());

    let tenant_a_results = ctx
        .execution_log
        .find_by_trigger_context(&tenant_a, trigger_context_id)
        .await
        .expect("tenant A lookup succeeds");
    assert_eq!(tenant_a_results.len(), 1);

    let tenant_b_results = ctx
        .execution_log
        .find_by_trigger_context(&tenant_b, trigger_context_id)
        .await
        .expect("tenant B lookup succeeds");
    assert!(tenant_b_results.is_empty());

    Ok(())
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_duplicate_execution_reuses_existing_result(
    #[future] context: Result<HookEngineTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let request_ctx = test_request_ctx();

    let hook_id = HookId::new("hook-postgres-retry").expect("valid hook id");
    let action_id = HookActionId::new("action-postgres-retry").expect("valid action id");
    let definition = HookDefinition::new(
        hook_id,
        "Retry-safe postgres hook",
        HookTriggerType::PostPush,
        vec![HookAction::new(action_id, HookActionType::Notification)],
    )
    .expect("definition should be valid");
    ctx.definition_repo
        .insert(&request_ctx, definition)
        .await
        .expect("insert succeeds");

    let trigger_context = HookTriggerContext::new(HookTriggerType::PostPush, &DefaultClock);
    let trigger_context_id = trigger_context.id();
    let first_results = ctx
        .service
        .execute(&request_ctx, trigger_context.clone())
        .await
        .expect("initial execution succeeds");
    let second_results = ctx
        .service
        .execute(&request_ctx, trigger_context)
        .await
        .expect("duplicate execution should reuse existing result");

    let first_result = first_results
        .first()
        .expect("expected first execution result");
    let second_result = second_results
        .first()
        .expect("expected duplicate execution result");
    assert_eq!(first_result.execution_id(), second_result.execution_id());

    let stored = ctx
        .execution_log
        .find_by_trigger_context(&request_ctx, trigger_context_id)
        .await
        .expect("lookup succeeds");
    assert_eq!(stored.len(), 1);

    Ok(())
}

#[rstest::rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_policy_audit_is_queryable_and_tenant_scoped(
    #[future] context: Result<HookEngineTestContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let tenant_a = test_request_ctx();
    let tenant_b = other_tenant_ctx(&tenant_a);
    let task_id = TaskId::new();
    let conversation_id = ConversationId::new();
    let hook_id = HookId::new("hook-postgres-policy-audit").expect("valid hook id");
    let action_id = HookActionId::new("action-postgres-policy-audit").expect("valid action id");
    let definition = HookDefinition::new(
        hook_id,
        "Postgres policy audit hook",
        HookTriggerType::PreToolUse,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .expect("definition should be valid");
    ctx.definition_repo
        .insert(&tenant_a, definition)
        .await
        .expect("insert succeeds");
    let action_executor = InMemoryHookActionExecutor::new();
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

    let service = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(ctx.definition_repo.clone()),
        action_executor: Arc::new(action_executor),
        execution_log: Arc::new(ctx.execution_log.clone()),
        policy_audit_repository: Arc::new(ctx.policy_audit.clone()),
        clock: Arc::new(DefaultClock),
    });
    let trigger_context = HookTriggerContext::new_with_timestamp(
        HookTriggerType::PreToolUse,
        HookExecutionScope::default()
            .with_task_id(task_id)
            .with_conversation_id(conversation_id)
            .with_metadata(json!({"tool_name": "read_file"})),
        DefaultClock.utc(),
    );
    let trigger_context_id = trigger_context.id();
    service
        .execute(&tenant_a, trigger_context)
        .await
        .expect("execution succeeds");

    assert_eq!(
        ctx.policy_audit
            .find_by_task(&tenant_a, task_id)
            .await
            .expect("query by task succeeds")
            .len(),
        1
    );
    assert_eq!(
        ctx.policy_audit
            .find_by_conversation(&tenant_a, conversation_id)
            .await
            .expect("query by conversation succeeds")
            .len(),
        1
    );
    assert_eq!(
        ctx.policy_audit
            .find_by_trigger_context(&tenant_a, trigger_context_id)
            .await
            .expect("query by trigger succeeds")
            .len(),
        1
    );
    assert!(
        ctx.policy_audit
            .find_by_task(&tenant_b, task_id)
            .await
            .expect("cross-tenant query succeeds")
            .is_empty()
    );
    Ok(())
}
