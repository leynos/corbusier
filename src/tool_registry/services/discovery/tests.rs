//! Unit tests for tool discovery and routing service.

#[path = "test_helpers.rs"]
mod test_helpers;

use super::ToolDiscoveryRoutingServiceError;
use crate::tool_registry::{
    adapters::{
        DenyAllPolicy, FailingPolicy, InMemoryMcpServerHost, memory::InMemoryMcpServerRegistry,
    },
    domain::{McpServerName, ToolCallRequest, ToolRegistryDomainError},
    services::McpServerLifecycleService,
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;
use std::sync::Arc;
use test_helpers::{
    TestBundle, assert_single_audit_stderr_path, bundle, call_read_file,
    call_read_file_expecting_error, discovery_with_policy, register_start_discover,
    register_start_with_stderr, setup_success_result, stdio_request, test_request_ctx,
};

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn discover_tools_persists_catalog(bundle: TestBundle) -> Result<()> {
    let TestBundle {
        host,
        lifecycle,
        discovery,
        ..
    } = bundle;
    let ctx = test_request_ctx();
    let server_id = register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;

    let entries = discovery.list_catalog(&ctx).await?;
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
    let ctx = test_request_ctx();
    let registered = lifecycle
        .register(&ctx, stdio_request("workspace_tools")?)
        .await?;
    assert!(matches!(
        discovery
            .discover_and_persist_tools(&ctx, registered.id())
            .await,
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
    let ctx = test_request_ctx();
    let server_id = register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;
    discovery.mark_tools_unavailable(&ctx, server_id).await?;

    let entries = discovery.list_catalog(&ctx).await?;
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
    let ctx = test_request_ctx();
    let server_id = register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;
    setup_success_result(&host)?;

    let result = call_read_file(&ctx, &discovery, json!({"path": "/tmp/test.txt"})).await?;
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
        host,
        lifecycle,
        discovery,
        ..
    } = bundle;
    let ctx = test_request_ctx();
    register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;

    let request = ToolCallRequest::new("nonexistent", json!({}), &DefaultClock);
    assert!(matches!(
        discovery.call_tool(&ctx, &request).await,
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
    let ctx = test_request_ctx();
    let server_id = register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;
    discovery.mark_tools_unavailable(&ctx, server_id).await?;

    let err =
        call_read_file_expecting_error(&ctx, &discovery, json!({"path": "/tmp/test.txt"})).await;
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
    let ctx = test_request_ctx();
    register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;

    let err = call_read_file_expecting_error(&ctx, &discovery, json!({})).await;
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
    let (disc, _catalog) =
        discovery_with_policy(&registry, &host, DenyAllPolicy::new("forbidden"), &clock);

    let ctx = test_request_ctx();
    register_start_discover(&host, &lifecycle, &disc, &ctx).await?;
    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert!(matches!(
        disc.call_tool(&ctx, &request).await,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::PolicyDenied { .. }
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn call_tool_policy_evaluation_failed() -> Result<()> {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let clock = Arc::new(DefaultClock);
    let lifecycle = McpServerLifecycleService::new(registry.clone(), host.clone(), clock.clone());
    let (disc, _catalog) =
        discovery_with_policy(&registry, &host, FailingPolicy::new("engine down"), &clock);

    let ctx = test_request_ctx();
    register_start_discover(&host, &lifecycle, &disc, &ctx).await?;
    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert!(matches!(
        disc.call_tool(&ctx, &request).await,
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
    let ctx = test_request_ctx();
    register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert!(discovery.call_tool(&ctx, &request).await.is_err());

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
    let ctx = test_request_ctx();
    register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;
    setup_success_result(&host)?;
    host.set_tool_call_stderr(
        McpServerName::new("workspace_tools")?,
        "read_file",
        bytes::Bytes::from("debug: opening file"),
    )?;

    let result = call_read_file(&ctx, &discovery, json!({"path": "/tmp/test.txt"})).await?;
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
    let ctx = test_request_ctx();
    register_start_discover(&host, &lifecycle, &discovery, &ctx).await?;
    setup_success_result(&host)?;
    call_read_file(&ctx, &discovery, json!({"path": "/tmp/test.txt"})).await?;
    assert_single_audit_stderr_path(&catalog, false)?;
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
    let ctx = test_request_ctx();
    let stderr_bytes = bytes::Bytes::from("server initialising...");
    let (server_id, captured_stderr) =
        register_start_with_stderr(&host, &lifecycle, &discovery, &ctx, stderr_bytes.clone())
            .await?;
    let captured = captured_stderr.expect("startup stderr should be captured");
    assert_eq!(captured, stderr_bytes);

    let metadata = discovery
        .store_startup_stderr(&ctx, server_id, captured)
        .await?;
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
#[case::non_object_params(
    json!({"type": "object"}), json!("not an object"), false,
)]
#[case::empty_required_array(
    json!({"type": "object", "required": []}), json!({}), true,
)]
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
