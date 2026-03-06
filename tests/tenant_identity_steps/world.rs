//! Shared world state for tenant identity BDD scenarios.

use corbusier::context::UserId;
use corbusier::tenant::{Tenant, TenantDomainError, TenantSlug};
use mockable::DefaultClock;
use rstest::fixture;

/// Scenario world for tenant identity behaviour tests.
pub struct TenantWorld {
    /// Raw slug string before validation.
    pub pending_slug: Option<String>,
    /// Raw display name before tenant creation.
    pub pending_display_name: Option<String>,
    /// Successfully created tenant.
    pub created_tenant: Option<Tenant>,
    /// Error from a failed creation attempt.
    pub creation_error: Option<TenantDomainError>,
}

impl TenantWorld {
    /// Creates a world with empty pending scenario state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pending_slug: None,
            pending_display_name: None,
            created_tenant: None,
            creation_error: None,
        }
    }

    /// Attempts to create a tenant from pending slug and display name.
    ///
    /// Stores the result in either `created_tenant` or `creation_error`.
    ///
    /// # Errors
    ///
    /// Returns an error if required pending state has not been set.
    pub fn attempt_creation(&mut self) -> Result<(), eyre::Report> {
        let slug_str = self
            .pending_slug
            .as_deref()
            .ok_or_else(|| eyre::eyre!("pending slug must be set before creation"))?;
        let display_name = self
            .pending_display_name
            .as_deref()
            .ok_or_else(|| eyre::eyre!("pending display name must be set before creation"))?;
        let clock = DefaultClock;

        // Reset prior results so created_tenant and creation_error remain
        // mutually exclusive across repeated creation attempts.
        self.created_tenant = None;
        self.creation_error = None;

        match TenantSlug::new(slug_str) {
            Ok(slug) => match Tenant::new(slug, display_name, UserId::new(), &clock) {
                Ok(tenant) => {
                    self.created_tenant = Some(tenant);
                }
                Err(err) => {
                    self.creation_error = Some(err);
                }
            },
            Err(err) => {
                self.creation_error = Some(err);
            }
        }
        Ok(())
    }
}

impl Default for TenantWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixture that creates a new scenario world.
#[fixture]
pub fn world() -> TenantWorld {
    TenantWorld::default()
}
