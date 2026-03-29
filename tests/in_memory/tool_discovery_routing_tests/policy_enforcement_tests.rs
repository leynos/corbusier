//! In-memory integration tests for hook-backed tool policy enforcement.

use super::{IntegrationContext, integration_ctx, read_file_tool, request_ctx, stdio_request};
use corbusier::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository, InMemoryHookPolicyAuditRepository,
};
use corbusier::hook_engine::domain::{
    HookAction, HookActionId, HookActionType, HookDefinition, HookId, HookTriggerType,
};
use corbusier::hook_engine::ports::HookPolicyAuditRepository;
use corbusier::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use corbusier::task::domain::TaskId;
use corbusier::tool_registry::adapters::HookBackedToolExecutionGovernance;
use corbusier::tool_registry::domain::{
    LogRetentionPolicy, McpServerName, ToolCallRequest, ToolRegistryDomainError,
};
use corbusier::tool_registry::services::{
    ServicePorts, ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError,
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;
use std::sync::Arc;

type InMemoryHookEngine = HookEngineService<
    InMemoryHookDefinitionRepository,
    InMemoryHookActionExecutor,
    InMemoryHookExecutionLogRepository,
    InMemoryHookPolicyAuditRepository,
    DefaultClock,
>;

type InMemoryGovernance = HookBackedToolExecutionGovernance<InMemoryHookEngine>;

type InMemoryGovernedDiscovery = ToolDiscoveryRoutingService<
    corbusier::tool_registry::adapters::memory::InMemoryToolCatalog,
    corbusier::tool_registry::adapters::memory::InMemoryMcpServerRegistry,
    corbusier::tool_registry::adapters::InMemoryMcpServerHost,
    InMemoryGovernance,
    corbusier::tool_registry::adapters::ObjectStoreLogAdapter,
    DefaultClock,
>;

struct GovernedInfrastructure {
    definition_repo: InMemoryHookDefinitionRepository,
    action_executor: InMemoryHookActionExecutor,
    policy_audit: InMemoryHookPolicyAuditRepository,
    governance: InMemoryGovernance,
}

fn build_governed_infrastructure() -> GovernedInfrastructure {
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let policy_audit = InMemoryHookPolicyAuditRepository::new();
    let hook_engine = HookEngineService::new(HookEngineServiceDeps {
        definition_repository: Arc::new(definition_repo.clone()),
        action_executor: Arc::new(action_executor.clone()),
        execution_log: Arc::new(execution_log),
        policy_audit_repository: Arc::new(policy_audit.clone()),
        clock: Arc::new(DefaultClock),
    });
    let governance = HookBackedToolExecutionGovernance::new(Arc::new(hook_engine));
    GovernedInfrastructure {
        definition_repo,
        action_executor,
        policy_audit,
        governance,
    }
}

fn build_discovery_with_governance(
    ctx: &IntegrationContext,
    governance: InMemoryGovernance,
) -> InMemoryGovernedDiscovery {
    ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: ctx.catalog.clone(),
            registry: ctx.registry.clone(),
            host: ctx.host.clone(),
            governance: Arc::new(governance),
            log_store: Arc::new(
                corbusier::tool_registry::adapters::ObjectStoreLogAdapter::in_memory(),
            ),
        },
        LogRetentionPolicy::default(),
        Arc::new(DefaultClock),
    )
}

async fn insert_deny_pre_tool_hook(
    ctx: &corbusier::context::RequestContext,
    definition_repo: &InMemoryHookDefinitionRepository,
    action_executor: &InMemoryHookActionExecutor,
) -> Result<()> {
    let action_id = HookActionId::new("deny-action")?;
    let definition = HookDefinition::new(
        HookId::new("deny-hook")?,
        "Deny hook",
        HookTriggerType::PreToolUse,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )?;
    definition_repo.insert(ctx, definition).await?;
    action_executor.set_output(
        action_id.as_str(),
        json!({
            "decision": "deny",
            "violation": {
                "code": "tool.blocked",
                "reason": "tool use is forbidden",
            }
        }),
    )?;
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn denied_call_leaves_queryable_policy_audit(
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let request_ctx = request_ctx();
    let task_id = TaskId::new();
    let GovernedInfrastructure {
        definition_repo,
        action_executor,
        policy_audit,
        governance,
    } = build_governed_infrastructure();
    let discovery = build_discovery_with_governance(&integration_ctx, governance);

    insert_deny_pre_tool_hook(&request_ctx, &definition_repo, &action_executor).await?;

    integration_ctx.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    let registered = integration_ctx
        .lifecycle
        .register(&request_ctx, stdio_request("workspace_tools")?)
        .await?;
    integration_ctx
        .lifecycle
        .start(&request_ctx, registered.id())
        .await?;
    discovery
        .discover_and_persist_tools(&request_ctx, registered.id())
        .await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock)
            .with_task_id(task_id);
    let err = discovery
        .call_tool(&request_ctx, &request)
        .await
        .expect_err("denied request should fail");
    assert!(matches!(
        err,
        ToolDiscoveryRoutingServiceError::Domain(ToolRegistryDomainError::PolicyDenied { .. })
    ));
    assert_eq!(
        policy_audit
            .find_by_task(&request_ctx, task_id)
            .await
            .expect("query by task succeeds")
            .len(),
        1
    );
    Ok(())
}
