//! Domain model for tenant identity and lifecycle.
//!
//! The tenant domain models first-class tenant boundaries with validated
//! identity primitives, lifecycle status, and owner-user binding. All
//! infrastructure concerns are kept outside the domain boundary.

mod error;
mod ids;
mod slug;
mod status;
mod tenant;

pub use error::{ParseTenantStatusError, TenantDomainError};
pub use ids::TenantId;
pub use slug::TenantSlug;
pub use status::TenantStatus;
pub use tenant::{PersistedTenantData, Tenant};

#[cfg(test)]
mod tests;
