//! Persistence adapters for the message module.
//!
//! This module provides concrete implementations of the [`MessageRepository`]
//! port, following hexagonal architecture principles. Adapters handle all
//! infrastructure concerns while the domain remains pure.
//!
//! # Available Adapters
//!
//! - [`memory::InMemoryMessageRepository`]: Thread-safe in-memory storage for
//!   unit testing
//! - [`postgres::PostgresMessageRepository`]: Production-grade `PostgreSQL`
//!   persistence using Diesel ORM
//!
//! # Audit Context
//!
//! The [`audit_context::AuditContext`] type provides correlation and causation
//! tracking for audit trails, propagated to database triggers via `PostgreSQL`
//! session settings.
//!
//! [`MessageRepository`]: crate::message::ports::repository::MessageRepository

pub mod audit_context;
pub mod memory;
pub mod models;
pub mod postgres;
pub mod schema;
