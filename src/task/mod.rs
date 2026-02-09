//! Task lifecycle management for Corbusier.
//!
//! This module implements roadmap item 1.2.1: creating internal task records
//! from external issue metadata and retrieving tasks by external issue
//! reference. The module follows hexagonal architecture:
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
