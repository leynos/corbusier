//! When steps for hook-backed tool policy enforcement scenarios.

use super::async_utils::run_async;
use super::world::HookPolicyWorld;
use corbusier::message::domain::ConversationId;
use corbusier::task::domain::TaskId;
use corbusier::tool_registry::domain::ToolCallRequest;
use corbusier::tool_registry::domain::ToolCallResult;
use corbusier::tool_registry::services::ToolDiscoveryRoutingServiceError;
use rstest_bdd_macros::when;
use serde_json::json;

const TEST_TOOL_NAME: &str = "read_file";

fn test_tool_params() -> serde_json::Value {
    json!({"path": "/tmp/test.txt"})
}

fn store_tool_call_outcome(
    world: &mut HookPolicyWorld,
    outcome: Result<ToolCallResult, ToolDiscoveryRoutingServiceError>,
) {
    match outcome {
        Ok(result) => {
            world.last_result = Some(result);
            world.last_error = None;
        }
        Err(err) => {
            world.last_error = Some(err);
            world.last_result = None;
        }
    }
}

fn execute_tool_call(
    world: &mut HookPolicyWorld,
    request: ToolCallRequest,
    conversation_id: Option<ConversationId>,
    task_id: Option<TaskId>,
) -> Result<(), eyre::Report> {
    world.last_request = Some(request.clone());
    world.last_conversation_id = conversation_id;
    world.last_task_id = task_id;
    store_tool_call_outcome(
        world,
        run_async(world.discovery.call_tool(&world.request_ctx, &request))?,
    );
    Ok(())
}

fn run_tool_call_for_world(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    let conversation_id = ConversationId::new();
    let request = ToolCallRequest::new(TEST_TOOL_NAME, test_tool_params(), &mockable::DefaultClock)
        .with_conversation_id(conversation_id);
    execute_tool_call(world, request, Some(conversation_id), None)
}

#[when("a tool call executes with conversation tracking")]
fn tool_call_executes_with_conversation(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    run_tool_call_for_world(world)
}

#[when("a tool call executes with task tracking")]
fn tool_call_executes_with_task(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    let task_id = TaskId::new();
    let request = ToolCallRequest::new(TEST_TOOL_NAME, test_tool_params(), &mockable::DefaultClock)
        .with_task_id(task_id);
    execute_tool_call(world, request, None, Some(task_id))
}

#[when("a successful tool call completes")]
fn successful_tool_call_completes(world: &mut HookPolicyWorld) -> Result<(), eyre::Report> {
    run_tool_call_for_world(world)
}
