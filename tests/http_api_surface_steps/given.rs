//! Given steps for HTTP API behaviour tests.

use super::world::{HttpApiWorld, required_field, required_str_field, world_mut};
use rstest_bdd_macros::given;

#[given("an authenticated HTTP API client")]
async fn authenticated_http_api_client(
    world: &mut Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let current_world = world_mut(world)?;
    eyre::ensure!(
        current_world.token.is_some(),
        "expected authenticated client token to be configured"
    );
    Ok(())
}

#[given("an unauthenticated HTTP API client")]
async fn unauthenticated_http_api_client(
    world: &mut Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let current_world = world_mut(world)?;
    current_world.token = None;
    Ok(())
}

#[given("I created a draft task through the API")]
async fn created_draft_task_through_api(
    world: &mut Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let current_world = world_mut(world)?;
    current_world
        .send(
            actix_web::test::TestRequest::post()
                .uri("/api/v1/tasks")
                .set_json(serde_json::json!({
                    "provider": "github",
                    "repository": "owner/repo",
                    "issue_number": 42,
                    "title": "Implement HTTP API"
                })),
        )
        .await?;
    let task_id = current_world.last_body.as_ref().map_or_else(
        || Err(eyre::eyre!("task id should be present")),
        |body| {
            Ok(
                required_str_field(required_field(required_field(body, "data"), "task"), "id")
                    .to_owned(),
            )
        },
    )?;
    current_world.task_id = Some(task_id);
    Ok(())
}
