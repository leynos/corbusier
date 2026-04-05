//! Behaviour tests for the HTTP API surface.

mod http_api_test_helpers;

#[path = "http_api_surface_steps/mod.rs"]
mod http_api_surface_steps_defs;

use http_api_surface_steps_defs::world::{HttpApiWorld, world};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "tests/features/http_api_surface.feature",
    name = "Create a conversation and append a message through HTTP"
)]
#[tokio::test(flavor = "multi_thread")]
async fn create_conversation_and_append_message(
    world: Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let _ = world?;
    Ok(())
}

#[scenario(
    path = "tests/features/http_api_surface.feature",
    name = "Create a task from issue metadata through HTTP"
)]
#[tokio::test(flavor = "multi_thread")]
async fn create_task_from_issue(
    world: Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let _ = world?;
    Ok(())
}

#[scenario(
    path = "tests/features/http_api_surface.feature",
    name = "Transition a task state through HTTP"
)]
#[tokio::test(flavor = "multi_thread")]
async fn transition_task_state(
    world: Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let _ = world?;
    Ok(())
}

#[scenario(
    path = "tests/features/http_api_surface.feature",
    name = "List tools and invoke a tool through HTTP"
)]
#[tokio::test(flavor = "multi_thread")]
async fn list_tools_and_invoke(
    world: Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let _ = world?;
    Ok(())
}

#[scenario(
    path = "tests/features/http_api_surface.feature",
    name = "Reject unauthenticated access"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_unauthenticated_access(
    world: Result<HttpApiWorld, eyre::Report>,
) -> Result<(), eyre::Report> {
    let _ = world?;
    Ok(())
}
