//! Port contracts for task lifecycle management.
//!
//! Ports define infrastructure-agnostic interfaces used by task services.

pub mod repository;

pub use repository::{TaskRepository, TaskRepositoryError, TaskRepositoryResult};
