//! Adapter implementations for MCP server lifecycle and registry ports.

pub mod memory;
pub mod postgres;

mod runtime;

pub use runtime::InMemoryMcpServerHost;
