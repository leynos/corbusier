//! Given steps for hook-backed tool policy enforcement scenarios.

use super::world::{HookPolicyWorld, run_async, stdio_request};
use corbusier::hook_engine::domain::{
    HookAction, HookActionId, HookActionType, HookDefinition, HookId, HookTriggerType,
};
use corbusier::tool_registry::domain::{McpServerName, McpToolDefinition};
use eyre::WrapErr;
use rstest_bdd_macros::given;
use serde_json::json;

fn read_file_tool() -> Result<McpToolDefinition, eyre::Report> {
    Ok(McpToolDefinition::new(
        "read_file",
        "Reads a file from the workspace",
        json!({"type": "object", "required": ["path"], "properties": {"path": {"type": "string"}}}),
    )?)
}

struct HookSetup<'a> {
    hook_id: &'a str,
    trigger: HookTriggerType,
    action_id: &'a str,
    output: serde_json::Value,
}

fn configure_hook(world: &mut HookPolicyWorld, setup: HookSetup<'_>) -> Result<(), eyre::Report> {
    let configured_action_id = HookActionId::new(setup.action_id)
        .wrap_err("build hook action identifier for scenario setup")?;
    let definition = HookDefinition::new(
        HookId::new(setup.hook_id).wrap_err("build hook identifier for scenario setup")?,
        format!("Hook {}", setup.hook_id),
        setup.trigger,
        vec![HookAction::new(
            configured_action_id.clone(),
            HookActionType::PolicyCheck,
        )],
    )
    .wrap_err("build hook definition for scenario setup")?;
    run_async(world.definition_repo.insert(&world.request_ctx, definition))
        .wrap_err("insert hook definition into scenario repository")?;
    world
        .action_executor
        .set_output(configured_action_id.as_str(), setup.output)
        .wrap_err("configure hook action output")?;
    Ok(())
}

fn prepare_runtime(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    world
        .host
        .set_tool_catalog(
            McpServerName::new("workspace_tools")?,
            vec![read_file_tool()?],
        )
        .wrap_err("configure in-memory host tool catalog")?;
    world
        .host
        .set_tool_call_result(
            McpServerName::new("workspace_tools")?,
            "read_file",
            json!({"content": "hello"}),
        )
        .wrap_err("configure tool call result")?;
    let registered = run_async(
        world
            .lifecycle
            .register(&world.request_ctx, stdio_request("workspace_tools")?),
    )
    .wrap_err("register workspace_tools server")?;
    run_async(world.lifecycle.start(&world.request_ctx, registered.id()))
        .wrap_err("start workspace_tools server")?;
    run_async(
        world
            .discovery
            .discover_and_persist_tools(&world.request_ctx, registered.id()),
    )
    .wrap_err("discover tools for workspace_tools")?;
    Ok(())
}

fn run_policy_setup(world: &mut HookPolicyWorld, setup: HookSetup) -> Result<(), eyre::Report> {
    configure_hook(world, setup)?;
    prepare_runtime(world)
}

#[given("a pre-tool-use policy hook permits tool calls")]
fn pre_tool_use_policy_permits(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    run_policy_setup(
        world,
        HookSetup {
            hook_id: "pre-tool-permit",
            trigger: HookTriggerType::PreToolUse,
            action_id: "pre-tool-allow-action",
            output: json!({"decision": "allow"}),
        },
    )
}

#[given("a pre-tool-use policy hook denies tool calls")]
fn pre_tool_use_policy_denies(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    configure_hook(
        world,
        HookSetup {
            hook_id: "pre-tool-deny",
            trigger: HookTriggerType::PreToolUse,
            action_id: "pre-tool-deny-action",
            output: json!({
                "decision": "deny",
                "violation": {
                    "code": "tool.blocked",
                    "reason": "tool use is forbidden",
                }
            }),
        },
    )?;
    prepare_runtime(world)
}

#[given("a post-tool-use policy hook records an allow decision")]
fn post_tool_use_policy_records_allow(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    run_policy_setup(
        world,
        HookSetup {
            hook_id: "post-tool-allow",
            trigger: HookTriggerType::PostToolUse,
            action_id: "post-tool-allow-action",
            output: json!({"decision": "allow"}),
        },
    )
}

#[given("a pre-tool-use policy hook emits an invalid payload")]
fn pre_tool_use_policy_emits_invalid_payload(
    world: &mut HookPolicyWorld,
) -> Result<(), eyre::Report> {
    run_policy_setup(
        world,
        HookSetup {
            hook_id: "pre-tool-invalid",
            trigger: HookTriggerType::PreToolUse,
            action_id: "pre-tool-invalid-action",
            output: json!({"status": "invalid"}),
        },
    )
}
