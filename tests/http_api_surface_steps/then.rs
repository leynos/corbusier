//! Then steps for HTTP API behaviour tests.

use super::world::{HttpApiWorld, required_field};
use rstest_bdd_macros::then;

fn last_body(world: &HttpApiWorld) -> &serde_json::Value {
    world
        .last_body
        .as_ref()
        .unwrap_or_else(|| panic!("response body should exist"))
}

#[then(r"the response status is {expected_status:u16}")]
async fn response_status_is(world: &mut HttpApiWorld, expected_status: u16) {
    assert_eq!(world.last_status, Some(expected_status));
}

#[then(r#"the response metadata version is "{expected_version}""#)]
async fn response_metadata_version_is(world: &mut HttpApiWorld, expected_version: String) {
    let body = last_body(world);
    let metadata = required_field(body, "metadata");
    assert_eq!(required_field(metadata, "version"), &expected_version);
    assert!(required_field(metadata, "request_id").is_string());
    assert!(required_field(metadata, "timestamp").is_string());
}

#[then(r"the conversation history includes {expected_count:usize} message")]
async fn conversation_history_includes_messages(world: &mut HttpApiWorld, expected_count: usize) {
    let body = last_body(world);
    let messages = required_field(required_field(body, "data"), "messages")
        .as_array()
        .unwrap_or_else(|| panic!("messages array should be present"));
    assert_eq!(messages.len(), expected_count);
}

#[then("the task is returned in the response")]
async fn task_is_returned_in_response(world: &mut HttpApiWorld) {
    let body = last_body(world);
    assert!(
        required_field(required_field(body, "data"), "task")
            .get("id")
            .is_some_and(serde_json::Value::is_string)
    );
}

#[then(r#"the task state is "{expected_state}""#)]
async fn task_state_is(world: &mut HttpApiWorld, expected_state: String) {
    let body = last_body(world);
    assert_eq!(
        required_field(
            required_field(required_field(body, "data"), "task"),
            "state"
        ),
        &expected_state
    );
}

#[then(r"the response includes {expected_tools:usize} tool")]
async fn response_includes_tool(world: &mut HttpApiWorld, expected_tools: usize) {
    let body = last_body(world);
    let tools = required_field(required_field(body, "data"), "tools")
        .as_array()
        .unwrap_or_else(|| panic!("tools array should be present"));
    assert_eq!(tools.len(), expected_tools);
}

#[then(r#"the tool call response names the tool "{tool_name}""#)]
async fn tool_call_response_names_tool(world: &mut HttpApiWorld, tool_name: String) {
    let body = last_body(world);
    assert_eq!(
        required_field(required_field(body, "data"), "tool_name"),
        &tool_name
    );
}
