//! Tenant identity and lifecycle bounded context.
//!
//! Provides domain primitives for tenant boundaries including validated
//! identity types, lifecycle status, and the tenant aggregate root. The
//! initial model supports one owning user per tenant while keeping user
//! and tenant identities distinct for future team and organisation tenants.

pub mod domain;

pub use domain::{
    ParseTenantStatusError, PersistedTenantData, Tenant, TenantDomainError, TenantId, TenantSlug,
    TenantStatus,
};
