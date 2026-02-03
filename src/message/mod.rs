//! Canonical message format and validation for Corbusier.
//!
//! This module implements the core message types, validation rules, and
//! schema versioning required by the Corbusier orchestration platform.
//!
//! # Architecture
//!
//! The module follows hexagonal architecture principles:
//!
//! - **Domain**: Pure domain types ([`domain::Message`], [`domain::Role`], [`domain::ContentPart`], etc.)
//! - **Ports**: Abstract trait interfaces ([`ports::repository::MessageRepository`], [`ports::validator::MessageValidator`])
//! - **Adapters**: Concrete implementations ([`adapters::memory::InMemoryMessageRepository`], [`adapters::postgres::PostgresMessageRepository`])
//! - **Validation**: Business rule enforcement at ingestion boundaries
//! - **Versioning**: Schema migration support for evolving event formats
//!
//! # Example
//!
//! ```
//! use corbusier::message::domain::{
//!     ContentPart, ConversationId, Message, Role, SequenceNumber, TextPart,
//! };
//! use corbusier::message::ports::validator::MessageValidator;
//! use corbusier::message::validation::service::DefaultMessageValidator;
//! use mockable::DefaultClock;
//!
//! let clock = DefaultClock;
//! let message = Message::builder(ConversationId::new(), Role::User, SequenceNumber::new(1))
//!     .with_content(ContentPart::Text(TextPart::new("Hello, Corbusier!")))
//!     .build(&clock)
//!     .expect("valid message");
//!
//! let validator = DefaultMessageValidator::new();
//! validator.validate(&message).expect("validation should pass");
//! ```

pub mod adapters;
pub mod domain;
pub mod error;
pub mod ports;
pub mod services;
pub mod validation;
pub mod versioning;

#[cfg(test)]
mod tests;
