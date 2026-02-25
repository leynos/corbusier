//! Agent backend registration and discovery for Corbusier.
//!
//! This module implements roadmap item 1.3.1: registering agent backends
//! with capability metadata, persisting registry entries, and discovering
//! registered backends via the registry API. The module follows hexagonal
//! architecture:
//!
//! - Domain types in [`domain`]
//! - Port contracts in [`ports`]
//! - Adapter implementations in [`adapters`]
//! - Orchestration services in [`services`]

pub mod adapters;
pub mod domain;
pub mod ports;
pub mod services;

#[cfg(test)]
mod tests;
