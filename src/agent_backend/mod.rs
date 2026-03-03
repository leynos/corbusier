//! Agent backend orchestration for Corbusier.
//!
//! This module currently implements roadmap items:
//!
//! - 1.3.1 backend registration and discovery
//! - 1.3.2 turn execution orchestration and session continuity
//!
//! The module follows hexagonal architecture:
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
