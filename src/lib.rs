//! Corbusier: AI agent orchestration platform.
//!
//! This crate provides the core functionality for orchestrating AI agents,
//! managing conversations, and coordinating tool execution across multiple
//! agent backends.
//!
//! # Architecture
//!
//! Corbusier follows hexagonal architecture principles:
//!
//! - **Domain**: Pure business logic with no infrastructure dependencies
//! - **Ports**: Abstract trait interfaces for external interactions
//! - **Adapters**: Concrete implementations of ports (database, APIs, etc.)
//!
//! # Modules
//!
//! - [`message`]: Canonical message format and validation
//! - [`task`]: Issue-to-task creation and lifecycle tracking

pub mod message;
pub mod task;
pub mod worker;
