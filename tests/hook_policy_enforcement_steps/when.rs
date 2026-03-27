//! When steps for hook-backed tool policy enforcement scenarios.

use super::world::{HookPolicyWorld, run_async};
use corbusier::message::domain::ConversationId;
use corbusier::task::domain::TaskId;
use corbusier::tool_registry::domain::ToolCallRequest;
use rstest_bdd_macros::when;
use serde_json::json;

fn run_tool_call_for_world(world: &mut HookPolicyWorld) {
    let conversation_id = ConversationId::new();
    let request = ToolCallRequest::new(
        "read_file",
        json!({"path": "/tmp/test.txt"}),
        &mockable::DefaultClock,
    )
    .with_conversation_id(conversation_id);
    world.last_request = Some(request.clone());
    world.last_conversation_id = Some(conversation_id);
    match run_async(world.discovery.call_tool(&world.request_ctx, &request)) {
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

#[when("a tool call executes with conversation tracking")]
fn tool_call_executes_with_conversation(world: &mut HookPolicyWorld) {
    run_tool_call_for_world(world);
}

#[when("a tool call executes with task tracking")]
fn tool_call_executes_with_task(world: &mut HookPolicyWorld) {
    let task_id = TaskId::new();
    let request = ToolCallRequest::new(
        "read_file",
        json!({"path": "/tmp/test.txt"}),
        &mockable::DefaultClock,
    )
    .with_task_id(task_id);
    world.last_request = Some(request.clone());
    world.last_task_id = Some(task_id);
    match run_async(world.discovery.call_tool(&world.request_ctx, &request)) {
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

#[when("a successful tool call completes")]
fn successful_tool_call_completes(world: &mut HookPolicyWorld) {
    run_tool_call_for_world(world);
}
