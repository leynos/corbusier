//! `PostgreSQL` integration tests for hook engine execution logs.

use corbusier::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
};
use corbusier::hook_engine::adapters::postgres::{
    HookExecutionPgPool, PostgresHookExecutionLogRepository,
};
use corbusier::hook_engine::domain::{
    HookAction, HookActionId, HookActionType, HookDefinition, HookId, HookTriggerContext,
    HookTriggerType,
};
use corbusier::hook_engine::ports::{HookEngine, HookExecutionLogRepository};
use corbusier::hook_engine::services::HookEngineService;
use corbusier::test_support::other_tenant_ctx;
use corbusier::test_support::test_request_ctx;
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use mockable::DefaultClock;
use rstest::fixture;
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
    DefaultClock,
>;

struct HookEngineTestContext {
    service: TestService,
    definition_repo: InMemoryHookDefinitionRepository,
    execution_log: PostgresHookExecutionLogRepository,
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
    let execution_log = PostgresHookExecutionLogRepository::new(pool);
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let service = HookEngineService::new(
        Arc::new(definition_repo.clone()),
        Arc::new(action_executor),
        Arc::new(execution_log.clone()),
        Arc::new(DefaultClock),
    );

    Ok(HookEngineTestContext {
        service,
        definition_repo,
        execution_log,
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
