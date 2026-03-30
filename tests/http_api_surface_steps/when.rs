//! When steps for HTTP API behaviour tests.

use super::world::HttpApiWorld;
use rstest_bdd_macros::when;
use serde_json::json;

#[when("I create a conversation through the API")]
async fn create_conversation(world: &mut HttpApiWorld) -> Result<(), eyre::Report> {
    world
        .send(actix_web::test::TestRequest::post().uri("/api/v1/conversations"))
        .await?;
    if let Some(conversation_id) = world.last_body.as_ref().and_then(|body| {
        body.get("data")
            .and_then(|data| data.get("conversation"))
            .and_then(|conversation| conversation.get("id"))
            .and_then(serde_json::Value::as_str)
    }) {
        world.conversation_id = Some(conversation_id.to_owned());
    }
    Ok(())
}

#[when(r#"I append the message "{message}" as "{role}""#)]
async fn append_message(
    world: &mut HttpApiWorld,
    message: String,
    role: String,
) -> Result<(), eyre::Report> {
    let conversation_id = world
        .conversation_id
        .clone()
        .unwrap_or_else(|| panic!("conversation id should be created first"));
    world
        .send(
            actix_web::test::TestRequest::post()
                .uri(&format!("/api/v1/conversations/{conversation_id}/messages"))
                .set_json(json!({
                    "role": role,
                    "content": [{ "type": "text", "text": message }]
                })),
        )
        .await
}

#[when("I request the conversation history")]
async fn request_conversation_history(world: &mut HttpApiWorld) -> Result<(), eyre::Report> {
    let conversation_id = world
        .conversation_id
        .clone()
        .unwrap_or_else(|| panic!("conversation id should be created first"));
    world
        .send(
            actix_web::test::TestRequest::get()
                .uri(&format!("/api/v1/conversations/{conversation_id}/history")),
        )
        .await
}

#[when(r#"I create a task from issue {issue_number:u64} in "{repository}""#)]
async fn create_task_from_issue(
    world: &mut HttpApiWorld,
    issue_number: u64,
    repository: String,
) -> Result<(), eyre::Report> {
    world
        .send(
            actix_web::test::TestRequest::post()
                .uri("/api/v1/tasks")
                .set_json(json!({
                    "provider": "github",
                    "repository": repository,
                    "issue_number": issue_number,
                    "title": "Implement HTTP API"
                })),
        )
        .await?;
    if let Some(task_id) = world.last_body.as_ref().and_then(|body| {
        body.get("data")
            .and_then(|data| data.get("task"))
            .and_then(|task| task.get("id"))
            .and_then(serde_json::Value::as_str)
    }) {
        world.task_id = Some(task_id.to_owned());
    }
    Ok(())
}

#[when(r#"I transition the task state to "{state}""#)]
async fn transition_task_state(
    world: &mut HttpApiWorld,
    state: String,
) -> Result<(), eyre::Report> {
    let task_id = world
        .task_id
        .clone()
        .unwrap_or_else(|| panic!("task id should be present"));
    world
        .send(
            actix_web::test::TestRequest::put()
                .uri(&format!("/api/v1/tasks/{task_id}/state"))
                .set_json(json!({ "state": state })),
        )
        .await
}

#[when("I list tools through the API")]
async fn list_tools(world: &mut HttpApiWorld) -> Result<(), eyre::Report> {
    world
        .send(actix_web::test::TestRequest::get().uri("/api/v1/tools"))
        .await
}

#[when(r#"I call the "{tool_name}" tool through the API"#)]
async fn call_tool(world: &mut HttpApiWorld, tool_name: String) -> Result<(), eyre::Report> {
    world
        .send(
            actix_web::test::TestRequest::post()
                .uri("/api/v1/tools/calls")
                .set_json(json!({
                    "tool_name": tool_name,
                    "parameters": { "path": "/tmp/example.txt" }
                })),
        )
        .await
}
