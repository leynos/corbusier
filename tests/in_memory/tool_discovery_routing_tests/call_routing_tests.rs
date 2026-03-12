//! Multi-server routing and duplicate rejection tests.

use super::{
    IntegrationContext, integration_ctx, read_file_tool, register_start_discover, request_ctx,
    search_code_tool,
};
use corbusier::context::RequestContext;
use corbusier::tool_registry::domain::{McpServerName, ToolCallRequest};
use eyre::Result;
use mockable::DefaultClock;
use rstest::rstest;
use serde_json::json;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn two_servers_route_correctly(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    let reg1 = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "file_tools",
        vec![read_file_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("file_tools")?,
        "read_file",
        json!({"content": "file contents"}),
    )?;

    let reg2 = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "search_tools",
        vec![search_code_tool()?],
    )
    .await?;
    integration_ctx.host.set_tool_call_result(
        McpServerName::new("search_tools")?,
        "search_code",
        json!({"matches": 3}),
    )?;

    let read_req =
        ToolCallRequest::new("read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock);
    assert_eq!(
        integration_ctx
            .discovery
            .call_tool(&request_ctx, &read_req)
            .await?
            .server_id(),
        reg1
    );

    let search_req = ToolCallRequest::new("search_code", json!({"query": "hello"}), &DefaultClock);
    assert_eq!(
        integration_ctx
            .discovery
            .call_tool(&request_ctx, &search_req)
            .await?
            .server_id(),
        reg2
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn duplicate_tool_name_is_rejected_during_discovery(
    request_ctx: RequestContext,
    integration_ctx: IntegrationContext,
) -> Result<()> {
    register_start_discover(
        &request_ctx,
        &integration_ctx,
        "file_tools",
        vec![read_file_tool()?],
    )
    .await?;
    let _err = register_start_discover(
        &request_ctx,
        &integration_ctx,
        "backup_tools",
        vec![read_file_tool()?],
    )
    .await
    .expect_err("second discovery should reject duplicate tool name");
    Ok(())
}
