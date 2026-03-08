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
    let transport = McpTransport::stdio("mcp-server")?;
    Ok(RegisterMcpServerRequest::new(name, transport))
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

async fn call_read_file_tool(
    discovery: &TestDiscoveryService,
    params: serde_json::Value,
) -> std::result::Result<
    crate::tool_registry::domain::ToolCallResult,
    ToolDiscoveryRoutingServiceError,
> {
    let request = ToolCallRequest::new("read_file", params, &DefaultClock);
    discovery.call_tool(&request).await
}

/// Calls `"read_file"` with the given `params` and asserts the call fails,
/// returning the unwrapped error for variant matching by the caller.
///
/// Panics if `call_tool` unexpectedly succeeds.
async fn call_read_file_expecting_error(
    discovery: &TestDiscoveryService,
    params: serde_json::Value,
) -> ToolDiscoveryRoutingServiceError {
    call_read_file_tool(discovery, params)
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

#[expect(
    clippy::indexing_slicing,
    reason = "asserts on first element after verifying len() == 1"
)]
#[expect(
    clippy::panic_in_result_fn,
    reason = "test helper uses assertions to validate audit state"
)]
fn assert_single_audit_stderr_path(
    catalog: &InMemoryToolCatalog,
    expected_some: bool,
) -> Result<()> {
    let audit_records = catalog.audit_records()?;
    assert_eq!(audit_records.len(), 1);
    assert_eq!(audit_records[0].stderr_log_path().is_some(), expected_some);
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::indexing_slicing,
    reason = "test asserts on first element after verifying len() == 1"
)]
async fn discover_tools_persists_catalog(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        ..
    } = bundle;

    let server_id = register_start_discover(&host, &lifecycle, &discovery).await?;

    let catalog_entries = discovery.list_catalog().await?;
    assert_eq!(catalog_entries.len(), 1);
    assert_eq!(catalog_entries[0].tool().name(), "read_file");
    assert!(catalog_entries[0].available());
    assert_eq!(catalog_entries[0].server_id(), server_id);

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

    let result = discovery.discover_and_persist_tools(registered.id()).await;

    assert!(matches!(
        result,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolQueryRequiresRunning { .. }
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::indexing_slicing,
    reason = "test asserts on first element after verifying len() == 1"
)]
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
    assert!(!entries[0].available());

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::indexing_slicing,
    reason = "test asserts on first element after verifying len() == 1"
)]
async fn call_tool_routes_to_correct_server(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
        ..
    } = bundle;

    let server_id = register_start_discover(&host, &lifecycle, &discovery).await?;
    host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello world"}),
    )?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = discovery.call_tool(&request).await?;

    assert!(result.outcome().is_success());
    assert_eq!(result.server_id(), server_id);
    assert_eq!(result.tool_name(), "read_file");

    let audit_records = catalog.audit_records()?;
    assert_eq!(audit_records.len(), 1);
    assert_eq!(audit_records[0].tool_name(), "read_file");

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
    let result = discovery.call_tool(&request).await;

    assert!(matches!(
        result,
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

    // Missing required 'path' parameter.
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
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let clock = Arc::new(DefaultClock);

    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let discovery = ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog,
            registry,
            host: host.clone(),
            policy: Arc::new(DenyAllPolicy::new("not authorised")),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock,
    );

    register_start_discover(&host, &lifecycle, &discovery).await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = discovery.call_tool(&request).await;

    assert!(matches!(
        result,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::PolicyDenied { .. }
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::indexing_slicing,
    reason = "test asserts on first element after verifying len() == 1"
)]
async fn call_tool_host_failure_still_records_audit(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        catalog,
        ..
    } = bundle;

    // Don't configure a call result -- the host will return ToolCallFailed.
    register_start_discover(&host, &lifecycle, &discovery).await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = discovery.call_tool(&request).await;

    assert!(result.is_err());

    let audit_records = catalog.audit_records()?;
    assert_eq!(audit_records.len(), 1);
    assert!(audit_records[0].outcome().is_failure());

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

    let result = call_read_file_tool(&discovery, json!({"path": "/tmp/test.txt"})).await?;

    assert!(result.outcome().is_success());

    assert_single_audit_stderr_path(&catalog, true)?;

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

    call_read_file_tool(&discovery, json!({"path": "/tmp/test.txt"})).await?;

    assert_single_audit_stderr_path(&catalog, false)?;

    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn store_startup_stderr_captures_log(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        lifecycle,
        discovery,
        ..
    } = bundle;

    let registered = lifecycle
        .register(stdio_request("workspace_tools")?)
        .await?;
    lifecycle.start(registered.id()).await?;

    let metadata = discovery
        .store_startup_stderr(registered.id(), bytes::Bytes::from("server starting up..."))
        .await?;

    assert!(metadata.object_path().contains("startup"));
    assert_eq!(metadata.server_id(), registered.id());

    Ok(())
}

#[cfg(test)]
mod validation_tests {
    use crate::tool_registry::domain::{ToolRegistryDomainError, validation::validate_parameters};
    use serde_json::json;

    #[test]
    fn valid_parameters_pass() {
        let schema = json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}});
        let params = json!({"path": "/tmp/test.txt"});
        assert!(validate_parameters(&schema, &params).is_ok());
    }

    #[test]
    fn missing_required_field_fails() {
        let schema = json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}});
        let params = json!({});
        let result = validate_parameters(&schema, &params);
        assert!(matches!(
            result,
            Err(ToolRegistryDomainError::SchemaValidationFailed { .. })
        ));
    }

    #[test]
    fn non_object_params_when_object_expected_fails() {
        let schema = json!({"type": "object"});
        let params = json!("not an object");
        let result = validate_parameters(&schema, &params);
        assert!(matches!(
            result,
            Err(ToolRegistryDomainError::SchemaValidationFailed { .. })
        ));
    }

    #[test]
    fn empty_required_array_passes() {
        let schema = json!({"type": "object", "required": []});
        let params = json!({});
        assert!(validate_parameters(&schema, &params).is_ok());
    }

    #[test]
    fn extra_fields_are_allowed() {
        let schema = json!({"type": "object", "required": ["path"]});
        let params = json!({"path": "/tmp/test.txt", "extra": "value"});
        assert!(validate_parameters(&schema, &params).is_ok());
    }
}
