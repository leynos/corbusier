//! When steps for tenant identity BDD scenarios.

use super::world::TenantWorld;
use rstest_bdd_macros::when;

#[when("the tenant is created")]
fn the_tenant_is_created(world: &mut TenantWorld) -> Result<(), eyre::Report> {
    world.attempt_creation()?;
    if let Some(ref err) = world.creation_error {
        return Err(eyre::eyre!("expected tenant creation to succeed: {err}"));
    }
    Ok(())
}

#[when("the tenant creation is attempted")]
fn the_tenant_creation_is_attempted(world: &mut TenantWorld) -> Result<(), eyre::Report> {
    world.attempt_creation()
}
