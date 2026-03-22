//! When steps for backend registration BDD scenarios.

use super::world::{BackendWorld, build_request, run_async};
use rstest_bdd_macros::when;

#[when("both backends are registered")]
fn register_both_backends(world: &mut BackendWorld) -> Result<(), eyre::Report> {
    for pending in &world.pending_backends {
        let request = build_request(&pending.name, &pending.provider);
        let result = run_async(world.service.register(&world.ctx, request));
        match result {
            Ok(registration) => {
                world.registered_backends.push(registration);
            }
            Err(err) => {
                return Err(eyre::eyre!("unexpected registration failure: {err}"));
            }
        }
    }
    Ok(())
}

#[when("a second backend with the same name is registered")]
fn register_duplicate_backend(world: &mut BackendWorld) -> Result<(), eyre::Report> {
    let pending = world
        .pending_backends
        .last()
        .ok_or_else(|| eyre::eyre!("no pending backend in scenario world"))?;
    let request = build_request(&pending.name, &pending.provider);
    world.last_register_result = Some(run_async(world.service.register(&world.ctx, request)));
    Ok(())
}

#[when("the backend is deactivated")]
fn deactivate_backend(world: &mut BackendWorld) -> Result<(), eyre::Report> {
    let registration = world
        .last_registered
        .as_ref()
        .ok_or_else(|| eyre::eyre!("no registered backend to deactivate"))?;
    run_async(world.service.deactivate(&world.ctx, registration.id()))
        .map_err(|err| eyre::eyre!("deactivation failed: {err}"))?;
    Ok(())
}

#[when("tenant A registers the backend")]
fn tenant_a_registers_backend(world: &mut BackendWorld) -> Result<(), eyre::Report> {
    let pending = world
        .pending_backends
        .last()
        .ok_or_else(|| eyre::eyre!("no pending backend in scenario world"))?;
    let request = build_request(&pending.name, &pending.provider);
    let registration = run_async(world.service.register(&world.ctx, request))
        .map_err(|err| eyre::eyre!("tenant A registration failed: {err}"))?;
    world.last_registered = Some(registration.clone());
    world.registered_backends.push(registration);
    Ok(())
}

#[when("tenant B registers a backend with the same name")]
fn tenant_b_registers_backend(world: &mut BackendWorld) -> Result<(), eyre::Report> {
    let pending = world
        .pending_backends
        .last()
        .ok_or_else(|| eyre::eyre!("no pending backend in scenario world"))?;
    let request = build_request(&pending.name, &pending.provider);
    let result = run_async(world.service.register(&world.other_ctx, request));
    if let Ok(registration) = &result {
        world.other_registered = Some(registration.clone());
    }
    world.other_register_result = Some(result);
    Ok(())
}
