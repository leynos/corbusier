//! When steps for tool discovery and routing BDD scenarios.

use corbusier::tool_registry::domain::ToolCallRequest;
use eyre::{WrapErr, eyre};
use mockable::DefaultClock;
use rstest_bdd_macros::when;

use super::world::ToolDiscoveryWorld;
use super::{request_from_world, run_async};

#[when("the server is registered and started")]
fn register_and_start_server(world: &mut ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    let registered = run_async(
        world
            .lifecycle()?
            .register(&world.request_ctx, request_from_world(world)?),
    )
    .wrap_err("registration should succeed")?;
    let start_result = run_async(
        world
            .lifecycle()?
            .start(&world.request_ctx, registered.id()),
    )
    .wrap_err("start should succeed")?;
    world.registered_server = Some(start_result.server);
    Ok(())
}

#[when("tools are discovered")]
fn discover_tools(world: &mut ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("server should be registered"))?;
    run_async(
        world
            .discovery()?
            .discover_and_persist_tools(&world.request_ctx, server.id()),
    )
    .wrap_err("tool discovery should succeed")?;
    Ok(())
}

#[when("the server is stopped")]
fn stop_server(world: &mut ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("server should be registered"))?;
    let stopped = run_async(world.lifecycle()?.stop(&world.request_ctx, server.id()))
        .wrap_err("stop should succeed")?;
    world.registered_server = Some(stopped);
    Ok(())
}

#[when("tools are marked unavailable")]
fn mark_tools_unavailable(world: &mut ToolDiscoveryWorld) -> Result<(), eyre::Report> {
    let server = world
        .registered_server
        .as_ref()
        .ok_or_else(|| eyre!("server should be registered"))?;
    run_async(
        world
            .discovery()?
            .mark_tools_unavailable(&world.request_ctx, server.id()),
    )
    .wrap_err("mark unavailable should succeed")?;
    Ok(())
}

#[when(r#"tool "{tool_name}" is called with parameters '{params}'"#)]
fn call_tool_with_params(
    world: &mut ToolDiscoveryWorld,
    tool_name: String,
    params: String,
) -> Result<(), eyre::Report> {
    let parameters: serde_json::Value =
        serde_json::from_str(&params).wrap_err("parameters should be valid JSON")?;
    let request = ToolCallRequest::new(&tool_name, parameters, &DefaultClock);
    match run_async(world.discovery()?.call_tool(&world.request_ctx, &request)) {
        Ok(_) => {
            world.last_call_succeeded = Some(true);
        }
        Err(err) => {
            world.last_call_succeeded = Some(false);
            world.last_error = Some(err);
        }
    }
    Ok(())
}
