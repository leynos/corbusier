//! Given steps for tenant identity BDD scenarios.

use super::world::TenantWorld;
use rstest_bdd_macros::given;

#[given(r#"a tenant slug "{slug}" with display name "{display_name}""#)]
fn a_tenant_slug_with_display_name(world: &mut TenantWorld, slug: String, display_name: String) {
    world.pending_slug = Some(slug);
    world.pending_display_name = Some(display_name);
}
