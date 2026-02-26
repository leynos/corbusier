//! Behavioural tests for slash-command orchestration.

use std::sync::Arc;

use corbusier::message::{
    adapters::memory::InMemorySlashCommandRegistry,
    domain::{SlashCommandError, SlashCommandExecution},
    services::SlashCommandService,
};
use eyre::{Result, eyre};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

#[derive(Default)]
struct SlashCommandWorld {
    service: Option<SlashCommandService<InMemorySlashCommandRegistry>>,
    first_execution: Option<SlashCommandExecution>,
    second_execution: Option<SlashCommandExecution>,
    last_error: Option<SlashCommandError>,
}

#[fixture]
fn world() -> SlashCommandWorld {
    SlashCommandWorld::default()
}

#[given("a slash command service with built-in commands")]
fn slash_command_service(world: &mut SlashCommandWorld) {
    world.service = Some(SlashCommandService::new(Arc::new(
        InMemorySlashCommandRegistry::new(),
    )));
    world.first_execution = None;
    world.second_execution = None;
    world.last_error = None;
}

#[when("I execute the slash command \"/task action=start issue=123\"")]
fn execute_valid_task_command(world: &mut SlashCommandWorld) {
    if let Some(service) = &world.service {
        match service.execute("/task action=start issue=123") {
            Ok(execution) => {
                world.first_execution = Some(execution);
                world.last_error = None;
            }
            Err(error) => {
                world.first_execution = None;
                world.last_error = Some(error);
            }
        }
    }
}

#[when("I execute the slash command \"/missing action=start\"")]
fn execute_missing_command(world: &mut SlashCommandWorld) {
    if let Some(service) = &world.service {
        world.last_error = service.execute("/missing action=start").err();
        world.first_execution = None;
    }
}

#[when("I execute the slash command \"/task issue=123\"")]
fn execute_missing_parameter(world: &mut SlashCommandWorld) {
    if let Some(service) = &world.service {
        world.last_error = service.execute("/task issue=123").err();
        world.first_execution = None;
    }
}

#[when("I execute the slash command twice \"/review action=sync include_summary=true\"")]
fn execute_command_twice(world: &mut SlashCommandWorld) {
    if let Some(service) = &world.service {
        world.first_execution = service
            .execute("/review action=sync include_summary=true")
            .ok();
        world.second_execution = service
            .execute("/review action=sync include_summary=true")
            .ok();
        world.last_error = None;
    }
}

#[then("the command expansion is recorded")]
fn command_expansion_recorded(world: &SlashCommandWorld) -> Result<()> {
    let execution = world
        .first_execution
        .as_ref()
        .ok_or_else(|| eyre!("expected first execution result"))?;

    assert_eq!(execution.expansion.command, "/task");
    assert_eq!(
        execution.expansion.parameters.get("action"),
        Some(&serde_json::Value::String("start".to_owned()))
    );
    Ok(())
}

#[then("a deterministic tool plan is produced")]
fn deterministic_tool_plan(world: &SlashCommandWorld) -> Result<()> {
    let execution = world
        .first_execution
        .as_ref()
        .ok_or_else(|| eyre!("expected first execution result"))?;

    assert!(!execution.planned_tool_calls.is_empty());
    assert_eq!(
        execution.planned_tool_calls.len(),
        execution.tool_call_audits.len()
    );
    Ok(())
}

#[then("the slash command fails with unknown command \"missing\"")]
fn unknown_command_failure(world: &SlashCommandWorld) -> Result<()> {
    let error = world
        .last_error
        .as_ref()
        .ok_or_else(|| eyre!("expected error"))?;

    assert_eq!(
        error,
        &SlashCommandError::UnknownCommand("missing".to_owned())
    );
    Ok(())
}

#[then("the slash command fails with missing parameter \"action\" for command \"task\"")]
fn missing_parameter_failure(world: &SlashCommandWorld) -> Result<()> {
    let error = world
        .last_error
        .as_ref()
        .ok_or_else(|| eyre!("expected error"))?;

    assert!(matches!(
        error,
        SlashCommandError::MissingRequiredParameter { command, parameter }
        if command == "task" && parameter == "action"
    ));
    Ok(())
}

#[then("both executions produce identical tool plans")]
fn identical_tool_plans(world: &SlashCommandWorld) -> Result<()> {
    let first = world
        .first_execution
        .as_ref()
        .ok_or_else(|| eyre!("expected first execution"))?;
    let second = world
        .second_execution
        .as_ref()
        .ok_or_else(|| eyre!("expected second execution"))?;

    assert_eq!(first.planned_tool_calls, second.planned_tool_calls);
    assert_eq!(first.tool_call_audits, second.tool_call_audits);
    Ok(())
}

#[scenario(
    path = "tests/features/slash_command.feature",
    name = "Valid command expands into a tool plan"
)]
#[tokio::test(flavor = "multi_thread")]
async fn valid_command(world: SlashCommandWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/slash_command.feature",
    name = "Unknown command is rejected"
)]
#[tokio::test(flavor = "multi_thread")]
async fn unknown_command(world: SlashCommandWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/slash_command.feature",
    name = "Missing required parameter is rejected"
)]
#[tokio::test(flavor = "multi_thread")]
async fn missing_parameter(world: SlashCommandWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/slash_command.feature",
    name = "Repeated command execution is deterministic"
)]
#[tokio::test(flavor = "multi_thread")]
async fn deterministic_execution(world: SlashCommandWorld) {
    let _ = world;
}
