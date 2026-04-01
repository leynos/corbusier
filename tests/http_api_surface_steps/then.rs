//! Then steps for HTTP API behaviour tests.

use super::world::{HttpApiWorld, required_field, world_mut, world_ref};
use rstest_bdd_macros::then;

fn last_body(
    world: &Result<HttpApiWorld, eyre::Report>,
) -> Result<&serde_json::Value, eyre::Report> {
    let current_world = world_ref(world)?;
    current_world
        .last_body
        .as_ref()
        .ok_or_else(|| eyre::eyre!("response body should exist"))
}

#[then(r"the response status is {expected_status:u16}")]
async fn response_status_is(
    world: &mut Result<HttpApiWorld, eyre::Report>,
    expected_status: u16,
) -> Result<(), eyre::Report> {
    let current_world = world_mut(world)?;
    eyre::ensure!(
        current_world.last_status == Some(expected_status),
        "expected response status {:?}, got {:?}",
        Some(expected_status),
        current_world.last_status
    );
    Ok(())
}

#[then(r#"the response metadata version is "{expected_version}""#)]
async fn response_metadata_version_is(
    world: &mut Result<HttpApiWorld, eyre::Report>,
    expected_version: String,
) -> Result<(), eyre::Report> {
    let body = last_body(world)?;
    let metadata = required_field(body, "metadata");
    eyre::ensure!(
        required_field(metadata, "version") == &expected_version,
        "expected metadata version {expected_version}, got {}",
        required_field(metadata, "version")
    );
    eyre::ensure!(
        required_field(metadata, "request_id").is_string(),
        "expected metadata.request_id to be a string"
    );
    eyre::ensure!(
        required_field(metadata, "timestamp").is_string(),
        "expected metadata.timestamp to be a string"
    );
    Ok(())
}

#[then("the conversation history includes {expected_count:usize} message")]
#[then("the conversation history includes {expected_count:usize} messages")]
async fn conversation_history_includes_messages(
    world: &mut Result<HttpApiWorld, eyre::Report>,
    expected_count: usize,
) -> Result<(), eyre::Report> {
    let body = last_body(world)?;
    let messages = required_field(required_field(body, "data"), "messages")
        .as_array()
        .ok_or_else(|| eyre::eyre!("messages array should be present"))?;
    eyre::ensure!(
        messages.len() == expected_count,
        "expected {expected_count} messages, got {}",
        messages.len()
    );
    Ok(())
}

#[then("the task is returned in the response")]
async fn task_is_returned_in_response(
    world: &mut Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let body = last_body(world)?;
    eyre::ensure!(
        required_field(required_field(body, "data"), "task")
            .get("id")
            .is_some_and(serde_json::Value::is_string),
        "expected response data.task.id to be a string"
    );
    Ok(())
}

#[then(r#"the task state is "{expected_state}""#)]
async fn task_state_is(
    world: &mut Result<HttpApiWorld, eyre::Report>,
    expected_state: String,
) -> Result<(), eyre::Report> {
    let body = last_body(world)?;
    eyre::ensure!(
        required_field(
            required_field(required_field(body, "data"), "task"),
            "state"
        ) == &expected_state,
        "expected task state {expected_state}, got {}",
        required_field(
            required_field(required_field(body, "data"), "task"),
            "state"
        )
    );
    Ok(())
}

#[then(r"the response includes {expected_tools:usize} tool")]
#[then(r"the response includes {expected_tools:usize} tools")]
async fn response_includes_tool(
    world: &mut Result<HttpApiWorld, eyre::Report>,
    expected_tools: usize,
) -> Result<(), eyre::Report> {
    let body = last_body(world)?;
    let tools = required_field(required_field(body, "data"), "tools")
        .as_array()
        .ok_or_else(|| eyre::eyre!("tools array should be present"))?;
    eyre::ensure!(
        tools.len() == expected_tools,
        "expected {expected_tools} tools, got {}",
        tools.len()
    );
    Ok(())
}

#[then(r#"the tool call response names the tool "{tool_name}""#)]
async fn tool_call_response_names_tool(
    world: &mut Result<HttpApiWorld, eyre::Report>,
    tool_name: String,
) -> Result<(), eyre::Report> {
    let body = last_body(world)?;
    eyre::ensure!(
        required_field(required_field(body, "data"), "tool_name") == &tool_name,
        "expected tool_name {tool_name}, got {}",
        required_field(required_field(body, "data"), "tool_name")
    );
    Ok(())
}
