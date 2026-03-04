//! Behaviour tests for tenant identity and domain primitives.

mod tenant_identity_steps;

use rstest_bdd_macros::scenario;
use tenant_identity_steps::world::{TenantWorld, world};

#[scenario(
    path = "tests/features/tenant_identity.feature",
    name = "Create a tenant with valid slug and display name"
)]
#[tokio::test(flavor = "multi_thread")]
async fn create_tenant_with_valid_slug(world: TenantWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tenant_identity.feature",
    name = "Reject tenant creation with invalid slug"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_invalid_slug(world: TenantWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tenant_identity.feature",
    name = "Reject tenant creation with empty display name"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_empty_display_name(world: TenantWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tenant_identity.feature",
    name = "Tenant slug normalises to lowercase"
)]
#[tokio::test(flavor = "multi_thread")]
async fn slug_normalises_to_lowercase(world: TenantWorld) {
    let _ = world;
}
