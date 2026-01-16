//! Unit tests for the message module.
//!
//! Tests are organised by domain concept, covering happy paths, error cases,
//! and edge cases for all public APIs.

mod adapters_tests;
mod audit_context_tests;
mod content_tests;
mod error_tests;
mod id_tests;
mod message_tests;
mod models_tests;
mod role_tests;
mod row_to_message_tests;
mod validation_config_tests;
mod validation_content_tests;
pub(crate) mod validation_fixtures;
mod validation_limits_tests;
mod validation_structure_tests;
mod versioning_tests;
