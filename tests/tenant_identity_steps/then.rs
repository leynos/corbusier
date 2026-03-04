//! Then steps for tenant identity BDD scenarios.

use super::world::TenantWorld;
use corbusier::tenant::TenantDomainError;
use rstest_bdd_macros::then;

#[then("the tenant has a unique identifier")]
fn tenant_has_unique_identifier(world: &TenantWorld) -> Result<(), eyre::Report> {
    let tenant = world
        .created_tenant
        .as_ref()
        .ok_or_else(|| eyre::eyre!("no tenant was created"))?;
    if tenant.id().into_inner().is_nil() {
        return Err(eyre::eyre!("tenant ID should not be nil"));
    }
    Ok(())
}

#[then(r#"the tenant slug is "{expected_slug}""#)]
fn tenant_slug_is(world: &TenantWorld, expected_slug: String) -> Result<(), eyre::Report> {
    let tenant = world
        .created_tenant
        .as_ref()
        .ok_or_else(|| eyre::eyre!("no tenant was created"))?;
    let actual = tenant.slug().as_str();
    if actual != expected_slug {
        return Err(eyre::eyre!(
            "expected slug '{expected_slug}', got '{actual}'"
        ));
    }
    Ok(())
}

#[then(r#"the tenant display name is "{expected_name}""#)]
fn tenant_display_name_is(world: &TenantWorld, expected_name: String) -> Result<(), eyre::Report> {
    let tenant = world
        .created_tenant
        .as_ref()
        .ok_or_else(|| eyre::eyre!("no tenant was created"))?;
    let actual = tenant.display_name();
    if actual != expected_name {
        return Err(eyre::eyre!(
            "expected display name '{expected_name}', got '{actual}'"
        ));
    }
    Ok(())
}

#[then(r#"the tenant status is "{expected_status}""#)]
fn tenant_status_is(world: &TenantWorld, expected_status: String) -> Result<(), eyre::Report> {
    let tenant = world
        .created_tenant
        .as_ref()
        .ok_or_else(|| eyre::eyre!("no tenant was created"))?;
    let actual = tenant.status().to_string();
    if actual != expected_status {
        return Err(eyre::eyre!(
            "expected status '{expected_status}', got '{actual}'"
        ));
    }
    Ok(())
}

#[then("tenant creation fails with a slug validation error")]
fn creation_fails_with_slug_error(world: &TenantWorld) -> Result<(), eyre::Report> {
    let err = world
        .creation_error
        .as_ref()
        .ok_or_else(|| eyre::eyre!("expected creation to fail, but it succeeded"))?;
    let is_slug_error = matches!(
        err,
        TenantDomainError::EmptySlug
            | TenantDomainError::InvalidSlug(_)
            | TenantDomainError::SlugBoundaryHyphen(_)
            | TenantDomainError::SlugConsecutiveHyphens(_)
            | TenantDomainError::SlugTooLong(_)
    );
    if !is_slug_error {
        return Err(eyre::eyre!("expected slug validation error, got: {err}"));
    }
    Ok(())
}

#[then("tenant creation fails with an empty display name error")]
fn creation_fails_with_empty_display_name(world: &TenantWorld) -> Result<(), eyre::Report> {
    let err = world
        .creation_error
        .as_ref()
        .ok_or_else(|| eyre::eyre!("expected creation to fail, but it succeeded"))?;
    if !matches!(err, TenantDomainError::EmptyDisplayName) {
        return Err(eyre::eyre!("expected EmptyDisplayName error, got: {err}"));
    }
    Ok(())
}
