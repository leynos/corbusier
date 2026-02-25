//! Then steps for backend registration BDD scenarios.

use super::world::{BackendWorld, run_async};
use corbusier::agent_backend::{
    ports::BackendRegistryError, services::BackendRegistryServiceError,
};
use rstest_bdd_macros::then;

#[then("listing all backends returns {count:usize} entries")]
fn list_all_returns_count(world: &mut BackendWorld, count: usize) -> Result<(), eyre::Report> {
    let all =
        run_async(world.service.list_all()).map_err(|err| eyre::eyre!("list_all failed: {err}"))?;
    world.last_list_all_result = Some(all.clone());
    if all.len() != count {
        return Err(eyre::eyre!(
            "expected {count} backends, found {}",
            all.len()
        ));
    }
    Ok(())
}

#[then(r#"the backend "{name}" can be found by name"#)]
fn backend_found_by_name(world: &mut BackendWorld, name: String) -> Result<(), eyre::Report> {
    let found = run_async(world.service.find_by_name(&name))
        .map_err(|err| eyre::eyre!("find_by_name failed: {err}"))?;
    if found.is_none() {
        return Err(eyre::eyre!("expected backend '{name}' to exist"));
    }
    Ok(())
}

#[then("registration fails with a duplicate name error")]
fn registration_fails_with_duplicate_name(world: &BackendWorld) -> Result<(), eyre::Report> {
    let result = world
        .last_register_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("missing registration result in scenario world"))?;
    if !matches!(
        result,
        Err(BackendRegistryServiceError::Repository(
            BackendRegistryError::DuplicateBackendName(_)
        ))
    ) {
        return Err(eyre::eyre!("expected duplicate name error, got {result:?}"));
    }
    Ok(())
}

#[then(r#"listing active backends does not include "{name}""#)]
fn active_listing_excludes(world: &mut BackendWorld, name: String) -> Result<(), eyre::Report> {
    let active = run_async(world.service.list_active())
        .map_err(|err| eyre::eyre!("list_active failed: {err}"))?;
    world.last_list_active_result = Some(active.clone());
    if active.iter().any(|b| b.name().as_str() == name) {
        return Err(eyre::eyre!("expected active listing to exclude '{name}'"));
    }
    Ok(())
}

#[then(r#"listing all backends still includes "{name}""#)]
fn all_listing_includes(world: &mut BackendWorld, name: String) -> Result<(), eyre::Report> {
    let all =
        run_async(world.service.list_all()).map_err(|err| eyre::eyre!("list_all failed: {err}"))?;
    if !all.iter().any(|b| b.name().as_str() == name) {
        return Err(eyre::eyre!("expected all-listing to include '{name}'"));
    }
    Ok(())
}
