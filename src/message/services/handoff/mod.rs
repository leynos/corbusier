//! Handoff service for orchestrating agent transitions.
//!
//! The `HandoffService` coordinates the lifecycle of agent handoffs,
//! ensuring context is preserved and proper audit trails are maintained.
//!
//! This module is split into submodules:
//! - [`params`]: Parameter types for initiating and completing handoffs
//! - [`conversions`]: Type conversions between session state and handoff status
//! - [`workflows`]: The [`HandoffService`] orchestration logic

mod conversions;
mod params;
mod workflows;

/// Parameter types for initiating and completing handoffs.
pub use params::{CompleteHandoffParams, ServiceInitiateParams};
/// Service for coordinating agent handoffs with context preservation.
pub use workflows::HandoffService;
