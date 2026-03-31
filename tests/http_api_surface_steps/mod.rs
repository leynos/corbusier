//! HTTP API surface test step definitions and world harness.
//!
//! This module provides the step definitions for HTTP API behavioural tests,
//! organised into the standard BDD pattern:
//! - `given`: Setup steps for establishing test preconditions
//! - `when`: Action steps for executing API operations
//! - `then`: Assertion steps for verifying responses
//! - `world`: Test harness and shared state (`HttpApiWorld`)

mod given;
mod then;
mod when;
pub mod world;
