//! Re-exported identifier types for the tenant domain.
//!
//! `TenantId` is defined in the cross-cutting [`crate::context::ids`] module
//! to avoid circular dependencies between context and tenant modules.

pub use crate::context::TenantId;
