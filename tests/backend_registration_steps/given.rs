//! Given steps for backend registration BDD scenarios.

use super::world::{BackendWorld, PendingBackend, build_request, run_async};
use eyre::WrapErr;
use rstest_bdd_macros::given;

#[given(r#"a backend named "{name}" from provider "{provider}""#)]
fn a_backend_named(world: &mut BackendWorld, name: String, provider: String) {
    world
        .pending_backends
        .push(PendingBackend { name, provider });
}

#[given("the backend has already been registered")]
fn backend_already_registered(world: &mut BackendWorld) -> Result<(), eyre::Report> {
    let pending = world
        .pending_backends
        .last()
        .ok_or_else(|| eyre::eyre!("no pending backend in scenario world"))?;
    let request = build_request(&pending.name, &pending.provider);
    let created = run_async(world.service.register(request))
        .wrap_err("register existing backend for duplicate scenario")?;
    world.last_registered = Some(created.clone());
    world.registered_backends.push(created);
    Ok(())
}

#[given(r#"a registered backend named "{name}" from provider "{provider}""#)]
fn registered_backend_named(
    world: &mut BackendWorld,
    name: String,
    provider: String,
) -> Result<(), eyre::Report> {
    let request = build_request(&name, &provider);
    let created =
        run_async(world.service.register(request)).wrap_err("register backend for scenario")?;
    world.last_registered = Some(created.clone());
    world.registered_backends.push(created);
    Ok(())
}
