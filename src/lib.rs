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
//! - [`context`]: Cross-cutting request context and identity types
//! - [`health`]: Health-check ports and the HTTP adapter used by the runtime
//! - [`http_api`]: HTTP API surface for conversations, tasks, and tools
//! - [`tenant`]: Tenant identity and lifecycle
//! - [`agent_backend`]: Agent backend registration and discovery
//! - [`hook_engine`]: Governance hook definition and execution
//! - [`message`]: Canonical message format and validation
//! - [`task`]: Issue-to-task creation and lifecycle tracking
//! - [`tool_registry`]: MCP server lifecycle management and tool discovery
//! - `test_support` (feature-gated): Shared fixtures and fakes for tests

pub mod context;
pub mod health;
pub mod http_api;
pub mod tenant;

pub mod agent_backend;
pub mod hook_engine;
pub mod message;
pub(crate) mod postgres_support;
pub mod task;
#[cfg(feature = "test-support")]
pub mod test_support;
pub mod tool_registry;
pub mod worker;
