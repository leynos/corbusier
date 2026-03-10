//! Discovery, availability, restart, audit trail, and stderr capture tests.

use super::{
    IntegrationContext, integration_ctx, read_file_tool, register_start_discover, request_ctx,
    stdio_request,
};
use corbusier::context::RequestContext;
use corbusier::tool_registry::{
    domain::{McpServerName, ToolCallRequest, ToolRegistryDomainError},
    services::ToolDiscoveryRoutingServiceError,
};
use eyre::Result;
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn discover_and_call_tool_end_to_end(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let server_id = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "workspace_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello world"}),
    )?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = integration_ctx
        .discovery
        .call_tool(&request_ctx, &request)
        .await?;
    assert!(result.outcome().is_success());
    assert_eq!(result.server_id(), server_id);
    assert_eq!(result.tool_name(), "read_file");

    let audits = integration_ctx
        .catalog
        .audit_records(request_ctx.tenant_id())?;
    assert_eq!(audits.len(), 1);
    assert_eq!(
        audits.first().expect("audit record").tool_name(),
        "read_file"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn tool_unavailable_after_stop(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let server_id = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "workspace_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx
        .lifecycle
        .stop(&request_ctx, server_id)
        .await?;
    integration_ctx
        .discovery
        .mark_tools_unavailable(&request_ctx, server_id)
        .await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert!(matches!(
        integration_ctx
            .discovery
            .call_tool(&request_ctx, &request)
            .await,
        Err(ToolDiscoveryRoutingServiceError::Domain(
            ToolRegistryDomainError::ToolUnavailable { .. }
        ))
    ));
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn rediscovery_after_restart(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let server_id = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "workspace_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;

    // Stop and mark unavailable.
    integration_ctx
        .lifecycle
        .stop(&request_ctx, server_id)
        .await?;
    integration_ctx
        .discovery
        .mark_tools_unavailable(&request_ctx, server_id)
        .await?;

    // Restart and rediscover.
    integration_ctx
        .lifecycle
        .start(&request_ctx, server_id)
        .await?;
    integration_ctx
        .discovery
        .discover_and_persist_tools(&request_ctx, server_id)
        .await?;

    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    let result = integration_ctx
        .discovery
        .call_tool(&request_ctx, &request)
        .await?;
    assert!(result.outcome().is_success());
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn audit_trail_accumulates(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    register_start_discover(
        &request_ctx,
        &integration_ctx,
        "workspace_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;

    for _ in 0..3 {
        let request =
            ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
        integration_ctx
            .discovery
            .call_tool(&request_ctx, &request)
            .await?;
    }
    assert_eq!(
        integration_ctx
            .catalog
            .audit_records(request_ctx.tenant_id())?
            .len(),
        3
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn stderr_captured_for_startup_and_tool_calls(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    // Configure startup stderr on the host before starting.
    let startup_bytes = bytes::Bytes::from("server initializing...");
    integration_ctx.host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![read_file_tool()?],
    )?;
    integration_ctx.host.set_startup_stderr(
        McpServerName::new("workspace_tools")?,
        startup_bytes.clone(),
    )?;

    // Start via lifecycle -- startup stderr flows through StartHostResult.
    let registered = integration_ctx
        .lifecycle
        .register(&request_ctx, stdio_request("workspace_tools")?)
        .await?;
    let start_result = integration_ctx
        .lifecycle
        .start(&request_ctx, registered.id())
        .await?;
    let captured = start_result
        .startup_stderr
        .expect("startup stderr should be captured");
    assert_eq!(captured, startup_bytes);

    // Persist startup stderr via discovery service.
    let startup_meta = integration_ctx
        .discovery
        .store_startup_stderr(&request_ctx, registered.id(), captured)
        .await?;
    assert!(startup_meta.object_path().contains("startup"));

    // Discover tools and configure tool call results.
    integration_ctx
        .discovery
        .discover_and_persist_tools(&request_ctx, registered.id())
        .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello"}),
    )?;
    integration_ctx.host.set_tool_call_stderr(
        McpServerName::new("workspace_tools")?,
        "read_file",
        bytes::Bytes::from("debug: reading file"),
    )?;

    // Call tool and verify audit trail references stderr.
    let request =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    integration_ctx
        .discovery
        .call_tool(&request_ctx, &request)
        .await?;

    let audits = integration_ctx
        .catalog
        .audit_records(request_ctx.tenant_id())?;
    assert_eq!(audits.len(), 1);
    let record = audits.first().expect("audit record");
    assert!(record.stderr_log_path().is_some());
    Ok(())
}
