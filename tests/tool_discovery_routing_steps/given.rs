//! Given steps for tool discovery and routing BDD scenarios.

use corbusier::tool_registry::domain::{McpServerName, McpToolDefinition};
use eyre::WrapErr;
use rstest_bdd_macros::given;
use serde_json::json;

use super::world::ToolDiscoveryWorld;

#[given(r#"a stdio MCP server named "{name}" with command "{command}""#)]
fn stdio_server_definition(world: &mut ToolDiscoveryWorld, name: String, command: String) {
    world.pending_name = Some(name);
    world.pending_command = Some(command);
}

#[given(r#"tool "{tool_name}" is available on that server"#)]
fn tool_available_on_server(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let tool = McpToolDefinition::new(
        &tool_name,
        format!("Tool {tool_name}"),
        json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    )
    .wrap_err("tool definition should be valid")?;

    world
        .host
        .set_tool_catalog(
            McpServerName::new(world.pending_name()?).wrap_err("valid pending name expected")?,
            vec![tool],
        )
        .wrap_err("catalogue setup should succeed")?;

    Ok(())
}

#[given(r#"calling tool "{tool_name}" on that server returns '{result}'"#)]
fn tool_call_result_configured(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
    result: String,
) -> Result<(), eyre::Report> {
    let value: serde_json::Value =
        serde_json::from_str(&result).wrap_err("result should be valid JSON")?;
    world
        .host
        .set_tool_call_result(
            McpServerName::new(world.pending_name()?).wrap_err("valid pending name expected")?,
            &tool_name,
            value,
        )
        .wrap_err("tool call result setup should succeed")?;
    Ok(())
}

#[given(r#"calling tool "{tool_name}" on that server produces stderr "{stderr}""#)]
fn tool_call_stderr_configured(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
    stderr: String,
) -> Result<(), eyre::Report> {
    world
        .host
        .set_tool_call_stderr(
            McpServerName::new(world.pending_name()?).wrap_err("valid pending name expected")?,
            &tool_name,
            bytes::Bytes::from(stderr),
        )
        .wrap_err("tool call stderr setup should succeed")?;
    Ok(())
}
