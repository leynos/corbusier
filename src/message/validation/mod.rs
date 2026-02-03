//! Message validation implementation.
//!
//! This module provides the default implementation of message validation,
//! including individual validation rules and the composite validator service.

pub mod handoff;
pub mod rules;
pub mod service;

pub use handoff::{
    HandoffValidationError, HandoffValidationResult, validate_handoff_can_cancel,
    validate_handoff_can_complete, validate_handoff_initiation,
    validate_session_can_initiate_handoff, validate_snapshot_for_handoff, validate_target_agent,
};
pub use service::DefaultMessageValidator;
