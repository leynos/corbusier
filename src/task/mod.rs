//! Task lifecycle management for Corbusier.
//!
//! This module implements roadmap items 1.2.1 and 1.2.2: creating internal task
//! records from external issue metadata, retrieving tasks by external issue
//! reference, and associating branch and pull request references with tasks.
//! Associating a pull request transitions the task state to `InReview`. The
//! module follows hexagonal architecture:
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
