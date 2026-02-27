//! Port contracts for agent backend registration and discovery.
//!
//! Ports define infrastructure-agnostic interfaces used by backend registry
//! services.

pub mod repository;

pub use repository::{BackendRegistryError, BackendRegistryRepository, BackendRegistryResult};
