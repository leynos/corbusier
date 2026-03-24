//! `PostgreSQL` integration tests for hook-backed tool policy enforcement.

use std::sync::Arc;

use corbusier::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
};
use corbusier::hook_engine::adapters::postgres::{
    HookExecutionPgPool, PostgresHookExecutionLogRepository, PostgresHookPolicyAuditRepository,
};
use corbusier::hook_engine::domain::{
    HookAction, HookActionId, HookActionType, HookDefinition, HookId, HookTriggerType,
};
use corbusier::hook_engine::ports::HookPolicyAuditRepository;
use corbusier::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use corbusier::task::domain::TaskId;
use corbusier::tool_registry::{
    adapters::{
        HookBackedToolExecutionGovernance, InMemoryMcpServerHost, ObjectStoreLogAdapter,
        postgres::{PostgresMcpServerRegistry, PostgresToolCatalog},
    },
    domain::{
        LogRetentionPolicy, McpServerName, McpToolDefinition, McpTransport, ToolCallRequest,
        ToolRegistryDomainError,
    },
    services::{
        McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
        ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError,
    },
};
use diesel::PgConnection;
use diesel::r2d2::ConnectionManager;
use eyre::Result;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use uuid::Uuid;

use crate::postgres::cluster::TemporaryDatabase;
use crate::postgres::helpers::{
    BoxError, PostgresCluster, TEMPLATE_DB, ensure_template, postgres_cluster, test_request_ctx,
};

type HookEngineTestService = HookEngineService<
    InMemoryHookDefinitionRepository,
    InMemoryHookActionExecutor,
    PostgresHookExecutionLogRepository,
    PostgresHookPolicyAuditRepository,
    DefaultClock,
>;

type TestLifecycleService =
    McpServerLifecycleService<PostgresMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

type TestDiscoveryService = ToolDiscoveryRoutingService<
    PostgresToolCatalog,
    PostgresMcpServerRegistry,
    InMemoryMcpServerHost,
    HookBackedToolExecutionGovernance<HookEngineTestService, PostgresHookPolicyAuditRepository>,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

struct PgGovernanceContext {
    host: Arc<InMemoryMcpServerHost>,
    lifecycle: TestLifecycleService,
    discovery: TestDiscoveryService,
    definition_repo: InMemoryHookDefinitionRepository,
    action_executor: InMemoryHookActionExecutor,
    policy_audit: PostgresHookPolicyAuditRepository,
    _temp_db: TemporaryDatabase,
}

async fn setup_context(cluster: PostgresCluster) -> Result<PgGovernanceContext, BoxError> {
    let db = cluster
        .temporary_database_from_template(
            &format!("tool_policy_enforcement_{}", Uuid::new_v4()),
            TEMPLATE_DB,
        )
        .await?;
    let manager = ConnectionManager::<PgConnection>::new(db.url());
    let pool: HookExecutionPgPool = diesel::r2d2::Pool::builder()
        .max_size(2)
        .build(manager)
        .map_err(|err| Box::new(err) as BoxError)?;

    let registry = Arc::new(PostgresMcpServerRegistry::new(pool.clone()));
    let catalog = Arc::new(PostgresToolCatalog::new(pool.clone()));
    let host = Arc::new(InMemoryMcpServerHost::new());
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = PostgresHookExecutionLogRepository::new(pool.clone());
    let policy_audit = PostgresHookPolicyAuditRepository::new(pool.clone());
    let hook_engine = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(definition_repo.clone()),
        action_executor: Arc::new(action_executor.clone()),
        execution_log: Arc::new(execution_log),
        policy_audit_repository: Arc::new(policy_audit.clone()),
        clock: Arc::new(DefaultClock),
    });
    let governance = HookBackedToolExecutionGovernance::new(
        Arc::new(hook_engine),
        Arc::new(policy_audit.clone()),
    );
    let discovery = ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog,
            registry: registry.clone(),
            host: host.clone(),
            policy: Arc::new(governance),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        Arc::new(DefaultClock),
    );
    let lifecycle = McpServerLifecycleService::new(registry, host.clone(), Arc::new(DefaultClock));

    Ok(PgGovernanceContext {
        host,
        lifecycle,
        discovery,
        definition_repo,
        action_executor,
        policy_audit,
        _temp_db: db,
    })
}

#[fixture]
async fn context(
    postgres_cluster: Result<PostgresCluster, BoxError>,
) -> Result<PgGovernanceContext, BoxError> {
    let cluster = postgres_cluster?;
    ensure_template(cluster).await?;
    setup_context(cluster).await
}

fn stdio_request(
    name: &str,
) -> Result<RegisterMcpServerRequest, corbusier::tool_registry::domain::ToolRegistryDomainError> {
    Ok(RegisterMcpServerRequest::new(
        name,
        McpTransport::stdio("mcp-server")?,
    ))
}

fn read_file_tool() -> Result<McpToolDefinition> {
    Ok(McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    )?)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn postgres_denied_tool_call_persists_policy_audit(
    #[future] context: Result<PgGovernanceContext, BoxError>,
) -> Result<(), BoxError> {
    let ctx = context.await?;
    let request_ctx = test_request_ctx();
    let task_id = TaskId::new();

    let action_id = HookActionId::new("deny-action").expect("valid action id");
    let definition = HookDefinition::new(
        HookId::new("deny-hook").expect("valid hook id"),
        "Deny hook",
        HookTriggerType::PreToolUse,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .expect("definition should be valid");
    ctx.definition_repo
        .insert(&request_ctx, definition)
        .await
        .expect("insert succeeds");
    ctx.action_executor
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

    ctx.host
        .set_tool_catalog(
            McpServerName::new("workspace_tools").expect("valid name"),
            vec![read_file_tool().expect("valid tool")],
        )
        .expect("catalog setup succeeds");
    let registered = ctx
        .lifecycle
        .register(
            &request_ctx,
            stdio_request("workspace_tools").expect("valid request"),
        )
        .await
        .expect("registration succeeds");
    ctx.lifecycle
        .start(&request_ctx, registered.id())
        .await
        .expect("start succeeds");
    ctx.discovery
        .discover_and_persist_tools(&request_ctx, registered.id())
        .await
        .expect("discovery succeeds");

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock)
            .with_task_id(task_id);
    let err = ctx
        .discovery
        .call_tool(&request_ctx, &request)
        .await
        .expect_err("denied tool call should fail");
    assert!(matches!(
        err,
        ToolDiscoveryRoutingServiceError::Domain(ToolRegistryDomainError::PolicyDenied { .. })
    ));
    assert_eq!(
        ctx.policy_audit
            .find_by_task(&request_ctx, task_id)
            .await
            .expect("query by task succeeds")
            .len(),
        1
    );
    Ok(())
}
