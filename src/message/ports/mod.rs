//! Port trait definitions for the message subsystem.
//!
//! Ports define the abstract interfaces that the domain requires from
//! infrastructure. Adapters implement these ports to connect the domain
//! to databases, external services, and other infrastructure.

pub mod repository;
pub mod validator;

pub use repository::MessageRepository;
pub use validator::{MessageValidator, ValidationConfig};
