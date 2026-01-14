//! Message validation implementation.
//!
//! This module provides the default implementation of message validation,
//! including individual validation rules and the composite validator service.

pub mod rules;
pub mod service;

pub use service::DefaultMessageValidator;
