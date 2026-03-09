//! Unit tests for tool discovery and routing service.

use super::{ServicePorts, ToolDiscoveryRoutingService, ToolDiscoveryRoutingServiceError};
use crate::tool_registry::{
    adapters::{
        AllowAllPolicy, DenyAllPolicy, InMemoryMcpServerHost, ObjectStoreLogAdapter,
        memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
    },
    domain::{
        LogRetentionPolicy, McpServerName, McpToolDefinition, McpTransport, ToolCallRequest,
        ToolRegistryDomainError,
    },
    services::{McpServerLifecycleService, RegisterMcpServerRequest},
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use serde_json::json;
use std::sync::Arc;

type TestLifecycleService =
    McpServerLifecycleService<InMemoryMcpServerRegistry, InMemoryMcpServerHost, DefaultClock>;
type TestDiscoveryService = ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    AllowAllPolicy,
    ObjectStoreLogAdapter,
    DefaultClock,
>;

struct TestBundle {
    host: Arc<InMemoryMcpServerHost>,
    lifecycle: TestLifecycleService,
    discovery: TestDiscoveryService,
    catalog: Arc<InMemoryToolCatalog>,
}

#[fixture]
fn bundle() -> TestBundle {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let clock = Arc::new(DefaultClock);
    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let discovery = ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: catalog.clone(),
            registry,
            host: host.clone(),
            policy: Arc::new(AllowAllPolicy),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock,
    );
    TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
    }
}

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

async fn register_start_discover<Pol: crate::tool_registry::ports::ToolPolicyEnforcer>(
    host: &InMemoryMcpServerHost,
    lifecycle: &TestLifecycleService,
    discovery: &ToolDiscoveryRoutingService<
        InMemoryToolCatalog,
        InMemoryMcpServerRegistry,
        InMemoryMcpServerHost,
        Pol,
        ObjectStoreLogAdapter,
        DefaultClock,
    >,
) -> Result<crate::tool_registry::domain::McpServerId> {
    host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    let registered = lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    lifecycle.start(registered.id()).await?;
    discovery
        .discover_and_persist_tools(registered.id())
        .await?;
    Ok(registered.id())
}

/// Registers, starts (with startup stderr), and discovers tools. Returns
/// the server identifier and the captured startup stderr bytes.
async fn register_start_with_stderr<Pol: crate::tool_registry::ports::ToolPolicyEnforcer>(
    host: &InMemoryMcpServerHost,
    lifecycle: &TestLifecycleService,
    discovery: &ToolDiscoveryRoutingService<
        InMemoryToolCatalog,
        InMemoryMcpServerRegistry,
        InMemoryMcpServerHost,
        Pol,
        ObjectStoreLogAdapter,
        DefaultClock,
    >,
    startup_stderr: bytes::Bytes,
) -> Result<(
    crate::tool_registry::domain::McpServerId,
    Option<bytes::Bytes>,
)> {
    host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    host.set_startup_stderr(McpServerName::new("workspace_tools")?, startup_stderr)?;
    let registered = lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    let start_result = lifecycle.start(registered.id()).await?;
    discovery
        .discover_and_persist_tools(registered.id())
        .await?;
    Ok((registered.id(), start_result.startup_stderr))
}

async fn call_read_file(
    discovery: &TestDiscoveryService,
    params: serde_json::Value,
) -> super::ToolDiscoveryRoutingServiceResult<crate::tool_registry::domain::ToolCallResult> {
    discovery
        .call_tool(&ToolCallRequest::new("read_file", params, &DefaultClock))
        .await
}

async fn call_read_file_expecting_error(
    discovery: &TestDiscoveryService,
    params: serde_json::Value,
) -> ToolDiscoveryRoutingServiceError {
    call_read_file(discovery, params)
        .await
        .expect_err("expected call_tool to return an error")
}

fn setup_success_result(host: &InMemoryMcpServerHost) -> Result<()> {
    host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;
    Ok(())
}

fn assert_single_audit_stderr_path(catalog: &InMemoryToolCatalog, expected_some: bool) {
    let audits = catalog
        .audit_records()
        .expect("failed to retrieve audit records");
    assert_eq!(audits.len(), 1);
    assert_eq!(
        audits
            .first()
            .expect("audit record")
            .stderr_log_path()
            .is_some(),
        expected_some
    );
}

/// Builds a discovery service wired to a custom policy adapter.
fn discovery_with_policy<Pol: crate::tool_registry::ports::ToolPolicyEnforcer + 'static>(
    registry: &Arc<InMemoryMcpServerRegistry>,
    host: &Arc<InMemoryMcpServerHost>,
    policy: Pol,
    clock: &Arc<DefaultClock>,
) -> ToolDiscoveryRoutingService<
    InMemoryToolCatalog,
    InMemoryMcpServerRegistry,
    InMemoryMcpServerHost,
    Pol,
    ObjectStoreLogAdapter,
    DefaultClock,
> {
    ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog: Arc::new(InMemoryToolCatalog::new()),
            registry: registry.clone(),
            host: host.clone(),
            policy: Arc::new(policy),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock.clone(),
    )
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn discover_tools_persists_catalog(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        ..
    } = bundle;
    let server_id = register_start_discover(&host, &lifecycle, &discovery).await?;

    let entries = discovery.list_catalog().await?;
    assert_eq!(entries.len(), 1);
    let first = entries.first().expect("expected single catalog entry");
    assert_eq!(first.tool().name(), "read_file");
    assert!(first.available());
    assert_eq!(first.server_id(), server_id);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn discover_tools_requires_running_server(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        lifecycle,
        discovery,
        ..
    } = bundle;
    let registered = lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    assert!(matches!(
        discovery.discover_and_persist_tools(registered.id()).await,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn mark_unavailable_updates_catalog(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        ..
    } = bundle;
    let server_id = register_start_discover(&host, &lifecycle, &discovery).await?;
    discovery.mark_tools_unavailable(server_id).await?;

    let entries = discovery.list_catalog().await?;
    assert_eq!(entries.len(), 1);
    assert!(!entries.first().expect("catalog entry").available());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_routes_to_correct_server(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
        ..
    } = bundle;
    let server_id = register_start_discover(&host, &lifecycle, &discovery).await?;
    setup_success_result(&host)?;

    let result = call_read_file(&discovery, json!({"path": "/tmp/test.txt"})).await?;
    assert!(result.outcome().is_success());
    assert_eq!(result.server_id(), server_id);
    assert_eq!(result.tool_name(), "read_file");

    let audits = catalog.audit_records()?;
    assert_eq!(audits.len(), 1);
    assert_eq!(
        audits.first().expect("audit record").tool_name(),
        "read_file"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_unknown_tool_returns_not_found(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        lifecycle,
        discovery,
        ..
    } = bundle;
    let registered = lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    lifecycle.start(registered.id()).await?;

    let request = ToolCallRequest::new("nonexistent", json!({}), &DefaultClock);
    assert!(matches!(
        discovery.call_tool(&request).await,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolNotFound(_)
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_unavailable_tool_returns_error(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        ..
    } = bundle;
    let server_id = register_start_discover(&host, &lifecycle, &discovery).await?;
    discovery.mark_tools_unavailable(server_id).await?;

    let err = call_read_file_expecting_error(&discovery, json!({"path": "/tmp/test.txt"})).await;
    assert!(matches!(
        err,
        ToolDiscoveryRoutingServiceError::Domain(ToolRegistryDomainError::ToolUnavailable { .. })
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_schema_validation_failure(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        ..
    } = bundle;
    register_start_discover(&host, &lifecycle, &discovery).await?;

    let err = call_read_file_expecting_error(&discovery, json!({})).await;
    assert!(matches!(
        err,
        ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::SchemaValidationFailed { .. }
        )
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_policy_denied() -> Result<()> {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let clock = Arc::new(DefaultClock);
    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let disc = discovery_with_policy(&registry, &host, DenyAllPolicy::new("forbidden"), &clock);

    register_start_discover(&host, &lifecycle, &disc).await?;
    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert!(matches!(
        disc.call_tool(&request).await,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::PolicyDenied { .. }
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_policy_evaluation_failed() -> Result<()> {
    use crate::tool_registry::adapters::FailingPolicy;
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let clock = Arc::new(DefaultClock);
    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let disc = discovery_with_policy(&registry, &host, FailingPolicy::new("engine down"), &clock);

    register_start_discover(&host, &lifecycle, &disc).await?;
    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert!(matches!(
        disc.call_tool(&request).await,
        Err(ToolDiscoveryRoutingServiceError::Policy(_))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_host_failure_still_records_audit(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
        ..
    } = bundle;
    register_start_discover(&host, &lifecycle, &discovery).await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert!(discovery.call_tool(&request).await.is_err());

    let audits = catalog.audit_records()?;
    assert_eq!(audits.len(), 1);
    assert!(audits.first().expect("audit record").outcome().is_failure());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_captures_stderr_in_log_store(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
        ..
    } = bundle;
    register_start_discover(&host, &lifecycle, &discovery).await?;
    setup_success_result(&host)?;
    host.set_tool_call_stderr(
        McpServerName::new("workspace_tools")?,
        "read_file",
        bytes::Bytes::from("debug: opening file"),
    )?;

    let result = call_read_file(&discovery, json!({"path": "/tmp/test.txt"})).await?;
    assert!(result.outcome().is_success());
    assert_single_audit_stderr_path(&catalog, true);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_without_stderr_has_no_log_path(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
        ..
    } = bundle;
    register_start_discover(&host, &lifecycle, &discovery).await?;
    setup_success_result(&host)?;
    call_read_file(&discovery, json!({"path": "/tmp/test.txt"})).await?;
    assert_single_audit_stderr_path(&catalog, false);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn startup_stderr_captured_end_to_end(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        ..
    } = bundle;
    let stderr_bytes = bytes::Bytes::from("server initialising...");
    let (server_id, captured_stderr) =
        register_start_with_stderr(&host, &lifecycle, &discovery, stderr_bytes.clone()).await?;
    let captured = captured_stderr.expect("startup stderr should be captured");
    assert_eq!(captured, stderr_bytes);

    let metadata = discovery.store_startup_stderr(server_id, captured).await?;
    assert!(metadata.object_path().contains("startup"));
    assert_eq!(metadata.server_id(), server_id);
    Ok(())
}

#[rstest]
#[case::valid_params(
    json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    json!({"path": "/tmp/test.txt"}), true,
)]
#[case::missing_required_field(
    json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    json!({}), false,
)]
#[case::non_object_params(json!({"type": "object"}), json!("not an object"), false)]
#[case::empty_required_array(json!({"type": "object", "required": []}), json!({}), true)]
#[case::extra_fields_allowed(
    json!({"type": "object", "required": ["path"]}),
    json!({"path": "/tmp/test.txt", "extra": "value"}), true,
)]
fn validate_parameters_cases(
    #[case] schema: serde_json::Value,
    #[case] params: serde_json::Value,
    #[case] expect_ok: bool,
) {
    use crate::tool_registry::domain::{ToolRegistryDomainError, validation::validate_parameters};
    let result = validate_parameters(&schema, &params);
    if expect_ok {
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    } else {
        assert!(
            matches!(
                result,
                Err(ToolRegistryDomainError::SchemaValidationFailed { .. })
            ),
            "expected SchemaValidationFailed, got {result:?}",
        );
    }
}
