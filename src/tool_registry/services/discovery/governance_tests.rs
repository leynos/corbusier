//! Unit tests for hook-backed tool governance in the discovery service.

use crate::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository, InMemoryHookPolicyAuditRepository,
};
use crate::hook_engine::domain::{
    HookAction, HookActionId, HookActionType, HookDefinition, HookId, HookTriggerType,
};
use crate::hook_engine::ports::HookPolicyAuditRepository;
use crate::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use crate::test_support::test_request_ctx;
use crate::tool_registry::adapters::{
    HookBackedToolExecutionGovernance, InMemoryMcpServerHost, ObjectStoreLogAdapter,
    memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
};
use crate::tool_registry::domain::{
    LogRetentionPolicy, McpServerName, McpToolDefinition, McpTransport, ToolCallRequest,
    ToolRegistryDomainError,
};
use crate::tool_registry::ports::ToolExecutionGovernance;
use crate::tool_registry::services::{
    McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts, ToolDiscoveryRoutingService,
    ToolDiscoveryRoutingServiceError,
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;
use std::sync::Arc;

type TestLifecycleService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;

type TestDiscoveryService<Gov> = ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    Gov,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

type TestGovernance = HookBackedToolExecutionGovernance<
    HookEngineService<
        InMemoryHookDefinitionRepository,
        InMemoryHookActionExecutor,
        InMemoryHookExecutionLogRepository,
        InMemoryHookPolicyAuditRepository,
        DefaultClock,
    >,
    InMemoryHookPolicyAuditRepository,
>;

fn stdio_request(name: &str) -> Result<RegisterMcpServerRequest, ToolRegistryDomainError> {
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

fn discovery_with_governance<Gov: ToolExecutionGovernance + 'static>(
    registry: &Arc<InMemoryMcpServerRegistry>,
    host: &Arc<InMemoryMcpServerHost>,
    governance: Gov,
    clock: &Arc<DefaultClock>,
) -> TestDiscoveryService<Gov> {
    ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: Arc::new(InMemoryToolCatalog::new()),
            registry: registry.clone(),
            host: host.clone(),
            policy: Arc::new(governance),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock.clone(),
    )
}

async fn register_start_discover<Gov: ToolExecutionGovernance>(
    host: &InMemoryMcpServerHost,
    lifecycle: &TestLifecycleService,
    discovery: &TestDiscoveryService<Gov>,
    ctx: &crate::context::RequestContext,
) -> Result<()> {
    host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    let registered = lifecycle
        .register(ctx, stdio_request("workspace_tools")?)
        .await?;
    lifecycle.start(ctx, registered.id()).await?;
    discovery
        .discover_and_persist_tools(ctx, registered.id())
        .await?;
    Ok(())
}

fn setup_success_result(host: &InMemoryMcpServerHost) -> Result<()> {
    host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;
    Ok(())
}

struct GovernanceTestFixture {
    ctx: crate::context::RequestContext,
    host: Arc<InMemoryMcpServerHost>,
    lifecycle: TestLifecycleService,
    definition_repo: InMemoryHookDefinitionRepository,
    action_executor: InMemoryHookActionExecutor,
    policy_audit: InMemoryHookPolicyAuditRepository,
    discovery: TestDiscoveryService<TestGovernance>,
}

impl GovernanceTestFixture {
    fn new() -> Self {
        let ctx = test_request_ctx();
        let clock = Arc::new(DefaultClock);
        let registry = Arc::new(InMemoryMcpServerRegistry::new());
        let host = Arc::new(InMemoryMcpServerHost::new());
        let lifecycle =
            McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());

        let definition_repo = InMemoryHookDefinitionRepository::new();
        let action_executor = InMemoryHookActionExecutor::new();
        let execution_log = InMemoryHookExecutionLogRepository::new();
        let policy_audit = InMemoryHookPolicyAuditRepository::new();
        let hook_engine = HookEngineService::new(HookEngineServiceDeps {
            definition_repository: Arc::new(definition_repo.clone()),
            action_executor: Arc::new(action_executor.clone()),
            execution_log: Arc::new(execution_log),
            policy_audit_repository: Arc::new(policy_audit.clone()),
            clock: clock.clone(),
        });
        let governance = HookBackedToolExecutionGovernance::new(
            Arc::new(hook_engine),
            Arc::new(policy_audit.clone()),
        );
        let discovery = discovery_with_governance(&registry, &host, governance, &clock);

        Self {
            ctx,
            host,
            lifecycle,
            definition_repo,
            action_executor,
            policy_audit,
            discovery,
        }
    }
}

fn tool_policy_definition(
    trigger_type: HookTriggerType,
    action_id: &HookActionId,
) -> HookDefinition {
    let (hook_id_str, label) = match trigger_type {
        HookTriggerType::PreToolUse => ("pre-tool-policy", "Pre-tool policy"),
        HookTriggerType::PostToolUse => ("post-tool-policy", "Post-tool policy"),
        _ => panic!("unsupported trigger type for tool policy definition fixture"),
    };
    HookDefinition::new(
        HookId::new(hook_id_str).expect("valid hook id"),
        label,
        trigger_type,
        vec![HookAction::new(
            action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .expect("tool policy definition should be valid")
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn denied_pre_tool_use_blocks_host_and_persists_policy_audit() -> Result<()> {
    let GovernanceTestFixture {
        ctx,
        host,
        lifecycle,
        definition_repo,
        action_executor,
        policy_audit,
        discovery,
    } = GovernanceTestFixture::new();
    let task_id = TaskId::new();
    let conversation_id = ConversationId::new();

    let action_id = HookActionId::new("deny-action").expect("valid action id");
    definition_repo
        .insert(
            &ctx,
            tool_policy_definition(HookTriggerType::PreToolUse, &action_id),
        )
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
        .expect("configure policy output should succeed");

    register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;
    setup_success_result(&host)?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock)
            .with_task_id(task_id)
            .with_conversation_id(conversation_id);
    let err = discovery
        .call_tool(&ctx, &request)
        .await
        .expect_err("denied tool call should fail");
    assert!(matches!(
        err,
        ToolDiscoveryRoutingServiceError::Domain(ToolRegistryDomainError::PolicyDenied { .. })
    ));
    assert_eq!(
        host.tool_call_count(&McpServerName::new("workspace_tools")?, "read_file")?,
        0
    );

    let events = policy_audit
        .find_by_task(&ctx, task_id)
        .await
        .expect("query by task should succeed");
    assert_eq!(events.len(), 1);
    let event = events.first().expect("expected policy audit event");
    assert_eq!(event.conversation_id(), Some(conversation_id));
    assert_eq!(
        event
            .violation()
            .expect("deny event should include violation")
            .reason(),
        "tool use is forbidden"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn invalid_post_tool_payload_does_not_fail_successful_tool_call() -> Result<()> {
    let GovernanceTestFixture {
        ctx,
        host,
        lifecycle,
        definition_repo,
        action_executor,
        policy_audit,
        discovery,
    } = GovernanceTestFixture::new();
    let conversation_id = ConversationId::new();

    let action_id = HookActionId::new("invalid-post-tool-action").expect("valid action id");
    definition_repo
        .insert(
            &ctx,
            tool_policy_definition(HookTriggerType::PostToolUse, &action_id),
        )
        .await
        .expect("insert post-tool policy definition should succeed");
    action_executor
        .set_output(action_id.as_str(), json!({"status": "invalid"}))
        .expect("configure invalid post-tool output should succeed");

    register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;
    setup_success_result(&host)?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock)
            .with_conversation_id(conversation_id);
    let result = discovery.call_tool(&ctx, &request).await?;

    assert!(result.outcome().is_success());
    let events = policy_audit
        .find_by_conversation(&ctx, conversation_id)
        .await
        .expect("query by conversation should succeed");
    assert!(
        events.is_empty(),
        "invalid post-tool payload should not persist audit events"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn post_tool_use_observation_records_audit_event() -> Result<()> {
    let GovernanceTestFixture {
        ctx,
        host,
        lifecycle,
        definition_repo,
        action_executor,
        policy_audit,
        discovery,
    } = GovernanceTestFixture::new();
    let conversation_id = ConversationId::new();

    let action_id = HookActionId::new("allow-action").expect("valid action id");
    definition_repo
        .insert(
            &ctx,
            tool_policy_definition(HookTriggerType::PostToolUse, &action_id),
        )
        .await
        .expect("insert post-tool definition should succeed");
    action_executor
        .set_output(action_id.as_str(), json!({"decision": "allow"}))
        .expect("configure post-tool output should succeed");

    register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;
    setup_success_result(&host)?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock)
            .with_conversation_id(conversation_id);
    let result = discovery
        .call_tool(&ctx, &request)
        .await
        .expect("allowed tool call should succeed");
    assert!(result.outcome().is_success());

    let by_conversation = policy_audit
        .find_by_conversation(&ctx, conversation_id)
        .await
        .expect("query by conversation should succeed");
    assert_eq!(by_conversation.len(), 1);
    let conversation_event = by_conversation
        .first()
        .expect("expected post-tool policy audit event");
    assert_eq!(
        conversation_event.trigger_type(),
        HookTriggerType::PostToolUse
    );

    let by_trigger = policy_audit
        .find_by_trigger_context(&ctx, conversation_event.trigger_context_id())
        .await
        .expect("query by trigger should succeed");
    assert_eq!(by_trigger.len(), 1);
    assert_eq!(
        by_trigger
            .first()
            .expect("expected trigger-scoped policy audit event")
            .decision()
            .as_str(),
        "allow"
    );
    Ok(())
}
