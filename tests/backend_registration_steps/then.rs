//! Then steps for backend registration BDD scenarios.

use super::world::{BackendWorld, run_async};
use corbusier::agent_backend::{
    ports::BackendRegistryError, services::BackendRegistryServiceError,
};
use rstest_bdd_macros::then;

#[then("listing all backends returns {count:usize} entries")]
fn list_all_returns_count(world: &mut BackendWorld, count: usize) -> Result<(), eyre::Report> {
    let all = run_async(world.service.list_all(&world.ctx))
        .map_err(|err| eyre::eyre!("list_all failed: {err}"))?;
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
    let found = run_async(world.service.find_by_name(&world.ctx, &name))
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
    let active = run_async(world.service.list_active(&world.ctx))
        .map_err(|err| eyre::eyre!("list_active failed: {err}"))?;
    world.last_list_active_result = Some(active.clone());
    if active.iter().any(|b| b.name().as_str() == name) {
        return Err(eyre::eyre!("expected active listing to exclude '{name}'"));
    }
    Ok(())
}

#[then(r#"listing all backends still includes "{name}""#)]
fn all_listing_includes(world: &mut BackendWorld, name: String) -> Result<(), eyre::Report> {
    let all = run_async(world.service.list_all(&world.ctx))
        .map_err(|err| eyre::eyre!("list_all failed: {err}"))?;
    if !all.iter().any(|b| b.name().as_str() == name) {
        return Err(eyre::eyre!("expected all-listing to include '{name}'"));
    }
    Ok(())
}

#[then("both tenants successfully register distinct backends with that name")]
fn both_tenants_register_distinct_backends(world: &BackendWorld) -> Result<(), eyre::Report> {
    let (tenant_a, tenant_b) = extract_two_registered(world)?;

    if tenant_a.id() == tenant_b.id() {
        return Err(eyre::eyre!(
            "tenant registrations must produce distinct IDs"
        ));
    }
    Ok(())
}

fn extract_two_registered(
    world: &BackendWorld,
) -> Result<
    (
        &corbusier::agent_backend::domain::AgentBackendRegistration,
        &corbusier::agent_backend::domain::AgentBackendRegistration,
    ),
    eyre::Report,
> {
    let tenant_a = world
        .last_registered
        .as_ref()
        .ok_or_else(|| eyre::eyre!("tenant A backend is missing"))?;
    let tenant_b = world
        .other_register_result
        .as_ref()
        .ok_or_else(|| eyre::eyre!("tenant B registration result is missing"))?
        .as_ref()
        .map_err(|err| eyre::eyre!("tenant B registration failed: {err}"))?;
    Ok((tenant_a, tenant_b))
}

fn lookup_backend_by_name(
    service: &super::world::TestRegistryService,
    ctx: &corbusier::context::RequestContext,
    name: &str,
    label: &str,
) -> Result<corbusier::agent_backend::domain::AgentBackendRegistration, eyre::Report> {
    run_async(service.find_by_name(ctx, name))
        .map_err(|err| eyre::eyre!("{label} lookup failed: {err}"))?
        .ok_or_else(|| eyre::eyre!("{label} backend not found"))
}

fn assert_backend_id_matches(
    found: &corbusier::agent_backend::domain::AgentBackendRegistration,
    expected: &corbusier::agent_backend::domain::AgentBackendRegistration,
    label: &str,
) -> Result<(), eyre::Report> {
    if found.id() != expected.id() {
        return Err(eyre::eyre!("{label} lookup returned the wrong backend"));
    }
    Ok(())
}

#[expect(
    clippy::too_many_arguments,
    reason = "BDD helper compares two tenant-scoped lookups against explicit expectations."
)]
fn assert_backends_by_name(
    service: &super::world::TestRegistryService,
    ctx_a: &corbusier::context::RequestContext,
    ctx_b: &corbusier::context::RequestContext,
    name: &str,
    expected_a: &corbusier::agent_backend::domain::AgentBackendRegistration,
    expected_b: &corbusier::agent_backend::domain::AgentBackendRegistration,
) -> Result<(), eyre::Report> {
    let found_a = lookup_backend_by_name(service, ctx_a, name, "tenant A")?;
    let found_b = lookup_backend_by_name(service, ctx_b, name, "tenant B")?;

    assert_backend_id_matches(&found_a, expected_a, "tenant A")?;
    assert_backend_id_matches(&found_b, expected_b, "tenant B")?;
    if found_a.id() == found_b.id() {
        return Err(eyre::eyre!("tenant lookups must return distinct backends"));
    }
    Ok(())
}

#[then("each tenant can find its own backend by name")]
fn each_tenant_finds_own_backend(world: &BackendWorld) -> Result<(), eyre::Report> {
    let pending = world
        .pending_backends
        .last()
        .ok_or_else(|| eyre::eyre!("no pending backend in scenario world"))?;
    let expected_a = world
        .last_registered
        .as_ref()
        .ok_or_else(|| eyre::eyre!("tenant A expected backend is missing"))?;
    let (_, expected_b) = extract_two_registered(world)?;
    assert_backends_by_name(
        &world.service,
        &world.ctx,
        &world.other_ctx,
        &pending.name,
        expected_a,
        expected_b,
    )
}
