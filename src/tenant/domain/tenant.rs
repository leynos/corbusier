//! Tenant aggregate root.

use crate::context::UserId;
use chrono::{DateTime, Utc};
use mockable::Clock;

use super::{TenantDomainError, TenantId, TenantSlug, TenantStatus};

/// Tenant aggregate root.
///
/// Represents a tenancy unit in the system. The initial model supports one
/// owning user per tenant while keeping user and tenant identities distinct
/// for future evolution to team and organisation tenants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tenant {
    id: TenantId,
    slug: TenantSlug,
    display_name: String,
    owner_user_id: UserId,
    status: TenantStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

/// Parameter object for reconstructing a persisted tenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedTenantData {
    /// Persisted tenant identifier.
    pub id: TenantId,
    /// Persisted tenant slug.
    pub slug: TenantSlug,
    /// Persisted display name.
    pub display_name: String,
    /// Persisted owning user identifier.
    pub owner_user_id: UserId,
    /// Persisted lifecycle status.
    pub status: TenantStatus,
    /// Persisted creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Persisted latest lifecycle timestamp.
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    /// Creates a new tenant with `Active` status.
    ///
    /// # Errors
    ///
    /// Returns [`TenantDomainError::EmptyDisplayName`] when the display name
    /// is empty after trimming.
    pub fn new(
        slug: TenantSlug,
        display_name: impl Into<String>,
        owner_user_id: UserId,
        clock: &impl Clock,
    ) -> Result<Self, TenantDomainError> {
        let raw_name: String = display_name.into();
        let trimmed = raw_name.trim();
        if trimmed.is_empty() {
            return Err(TenantDomainError::EmptyDisplayName);
        }

        let timestamp = clock.utc();
        Ok(Self {
            id: TenantId::new(),
            slug,
            display_name: trimmed.to_owned(),
            owner_user_id,
            status: TenantStatus::Active,
            created_at: timestamp,
            updated_at: timestamp,
        })
    }

    /// Reconstructs a tenant from persisted storage.
    ///
    /// # Errors
    ///
    /// Returns [`TenantDomainError::EmptyDisplayName`] when the persisted
    /// display name is empty after trimming.
    pub fn from_persisted(data: PersistedTenantData) -> Result<Self, TenantDomainError> {
        let trimmed = data.display_name.trim();
        if trimmed.is_empty() {
            return Err(TenantDomainError::EmptyDisplayName);
        }

        Ok(Self {
            id: data.id,
            slug: data.slug,
            display_name: trimmed.to_owned(),
            owner_user_id: data.owner_user_id,
            status: data.status,
            created_at: data.created_at,
            updated_at: data.updated_at,
        })
    }

    /// Returns the tenant identifier.
    #[must_use]
    pub const fn id(&self) -> TenantId {
        self.id
    }

    /// Returns the tenant slug.
    #[must_use]
    pub const fn slug(&self) -> &TenantSlug {
        &self.slug
    }

    /// Returns the display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Returns the owning user identifier.
    #[must_use]
    pub const fn owner_user_id(&self) -> UserId {
        self.owner_user_id
    }

    /// Returns the tenant lifecycle status.
    #[must_use]
    pub const fn status(&self) -> TenantStatus {
        self.status
    }

    /// Returns the creation timestamp.
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns the latest lifecycle timestamp.
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Suspends the tenant.
    pub fn suspend(&mut self, clock: &impl Clock) {
        self.status = TenantStatus::Suspended;
        self.touch(clock);
    }

    /// Reactivates the tenant.
    pub fn reactivate(&mut self, clock: &impl Clock) {
        self.status = TenantStatus::Active;
        self.touch(clock);
    }

    /// Updates the `updated_at` timestamp to the current clock time.
    fn touch(&mut self, clock: &impl Clock) {
        self.updated_at = clock.utc();
    }
}
