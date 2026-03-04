//! MCP server lifecycle management and tool registry for Corbusier.
//!
//! This module implements roadmap item 2.1.1: registering MCP server
//! configurations, managing server lifecycle (`start`, `stop`, health
//! reporting), and querying available tools from running servers. The module
//! follows hexagonal architecture:
//!
//! - Domain types in [`domain`]
//! - Port contracts in [`ports`]
//! - Adapter implementations in [`adapters`]
//! - Orchestration services in [`services`]

pub mod adapters;
pub mod domain;
pub mod ports;
pub mod services;
