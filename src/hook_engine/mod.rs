//! Hook engine for executing governance automation.
//!
//! This module implements roadmap item 2.3.1 by providing hook definitions,
//! trigger evaluation, execution orchestration, and persistence of hook
//! execution results.

pub mod adapters;
pub mod domain;
pub mod ports;
pub mod services;

#[cfg(test)]
mod tests;
