//! Error types for agent turn orchestration.

use crate::agent_backend::{
    domain::{BackendId, TurnSessionDomainError},
    ports::{
        AgentRuntimeError, BackendRegistryError, ToolRoutingError, TurnSessionRepositoryError,
    },
};
use thiserror::Error;

/// Service-level errors for turn orchestration.
#[derive(Debug, Error)]
pub enum AgentTurnOrchestrationError {
    /// Backend was not found in registry.
    #[error("backend {0} not found")]
    BackendNotFound(BackendId),

    /// Backend is registered but inactive.
    #[error("backend {0} is inactive")]
    BackendInactive(BackendId),

    /// Session TTL configuration is invalid.
    #[error("session ttl must be positive seconds, got {0}")]
    InvalidSessionTtl(i64),

    /// Backend registry operation failed.
    #[error(transparent)]
    BackendRegistry(#[from] BackendRegistryError),

    /// Runtime adapter operation failed.
    #[error(transparent)]
    Runtime(#[from] AgentRuntimeError),

    /// Session repository operation failed.
    #[error(transparent)]
    SessionRepository(#[from] TurnSessionRepositoryError),

    /// Session-domain validation failed.
    #[error(transparent)]
    SessionDomain(#[from] TurnSessionDomainError),

    /// Tool routing failed for one call.
    #[error("tool routing failed for call {call_id} ({tool_name}): {source}")]
    ToolRouting {
        /// Deterministic call identifier.
        call_id: String,
        /// Tool name associated with the failure.
        tool_name: String,
        /// Underlying routing error.
        source: ToolRoutingError,
    },
}

/// Result type for orchestration operations.
pub type AgentTurnOrchestrationResult<T> = Result<T, AgentTurnOrchestrationError>;
