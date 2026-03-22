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
use corbusier::hook_engine::services::HookEngineService;
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

type InMemoryGovernance =
    HookBackedToolExecutionGovernance<InMemoryHookEngine, InMemoryHookPolicyAuditRepository>;

type InMemoryGovernedDiscovery = ToolDiscoveryRoutingService<
    corbusier::tool_registry::adapters::memory::InMemoryToolCatalog,
    corbusier::tool_registry::adapters::memory::InMemoryMcpServerRegistry,
    corbusier::tool_registry::adapters::InMemoryMcpServerHost,
    InMemoryGovernance,
    corbusier::tool_registry::adapters::ObjectStoreLogAdapter,
    DefaultClock,
>;

fn build_discovery_with_governance(
    ctx: &IntegrationContext,
    governance: InMemoryGovernance,
) -> InMemoryGovernedDiscovery {
    ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: ctx.catalog.clone(),
            registry: ctx.registry.clone(),
            host: ctx.host.clone(),
            policy: Arc::new(governance),
            log_store: Arc::new(
                corbusier::tool_registry::adapters::ObjectStoreLogAdapter::in_memory(),
            ),
        },
        LogRetentionPolicy::default(),
        Arc::new(DefaultClock),
    )
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn denied_call_leaves_queryable_policy_audit(
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let request_ctx = request_ctx();
    let task_id = TaskId::new();
    let definition_repo = InMemoryHookDefinitionRepository::new();
    let action_executor = InMemoryHookActionExecutor::new();
    let execution_log = InMemoryHookExecutionLogRepository::new();
    let policy_audit = InMemoryHookPolicyAuditRepository::new();
    let hook_engine = HookEngineService::new(
        Arc::new(definition_repo.clone()),
        Arc::new(action_executor.clone()),
        Arc::new(execution_log),
        Arc::new(policy_audit.clone()),
        Arc::new(DefaultClock),
    );
    let governance = HookBackedToolExecutionGovernance::new(
        Arc::new(hook_engine),
        Arc::new(policy_audit.clone()),
    );
    let discovery = build_discovery_with_governance(&integration_ctx, governance);

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
    definition_repo
        .insert(&request_ctx, definition)
        .await
        .expect("insert succeeds");
    action_executor
        .set_output(
            action_id.as_str(),
            json!({
                "decision": "deny",
                "reason": "tool use is forbidden",
            }),
        )
        .expect("configure output succeeds");

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
