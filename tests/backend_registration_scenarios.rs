//! Behaviour tests for agent backend registration and discovery.

mod backend_registration_steps;

use backend_registration_steps::world::{BackendWorld, world};
use rstest_bdd_macros::scenario;

#[scenario(
    path = "tests/features/backend_registration.feature",
    name = "Register two backends and list them"
)]
#[tokio::test(flavor = "multi_thread")]
async fn register_two_and_list(world: BackendWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/backend_registration.feature",
    name = "Reject duplicate backend name"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_duplicate_name(world: BackendWorld) {
    let _ = world;
}

#[scenario(
    path = "tests/features/backend_registration.feature",
    name = "Deactivate a backend and exclude from active listing"
)]
#[tokio::test(flavor = "multi_thread")]
async fn deactivate_excludes_from_active(world: BackendWorld) {
    let _ = world;
}
