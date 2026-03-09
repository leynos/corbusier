//! Cross-cutting request context and identity types.
//!
//! This module provides the [`RequestContext`] struct that carries tenant
//! identity, distributed tracing identifiers, and authenticated principal
//! information through every repository and service call. It also provides
//! the newtype identifiers used across bounded contexts.

pub mod ids;
mod request_context;

pub use ids::{CausationId, CorrelationId, SessionId, TenantId, UserId};
pub use request_context::RequestContext;

#[cfg(test)]
mod tests;
