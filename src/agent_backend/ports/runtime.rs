//! Runtime port for agent turn execution.

use crate::agent_backend::domain::{
    AgentBackendRegistration, TurnExecutionRequest, TurnExecutionResult,
};
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Result type for agent runtime operations.
pub type AgentRuntimeResult<T> = Result<T, AgentRuntimeError>;

/// Port for creating backend sessions and executing turns.
#[async_trait]
pub trait AgentRuntimePort: Send + Sync {
    /// Creates a backend-native session identifier for a conversation.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError`] when session creation fails.
    async fn create_session(
        &self,
        backend: &AgentBackendRegistration,
        conversation_id: Uuid,
    ) -> AgentRuntimeResult<String>;

    /// Executes one turn against the provided backend session.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError`] when turn execution fails.
    async fn execute_turn(
        &self,
        backend: &AgentBackendRegistration,
        runtime_session_id: &str,
        request: &TurnExecutionRequest,
    ) -> AgentRuntimeResult<TurnExecutionResult>;
}

/// Errors returned by agent runtime adapters.
#[derive(Debug, Error)]
pub enum AgentRuntimeError {
    /// Session creation failed.
    #[error("runtime session creation failed: {0}")]
    SessionCreationFailed(String),

    /// The runtime session ID is invalid.
    #[error("invalid runtime session id")]
    InvalidRuntimeSessionId,

    /// Turn execution failed.
    #[error("runtime turn execution failed: {0}")]
    TurnExecutionFailed(String),

    /// Infrastructure failure from the runtime adapter.
    #[error("runtime infrastructure error: {0}")]
    Infrastructure(Arc<dyn std::error::Error + Send + Sync>),
}

impl AgentRuntimeError {
    /// Wraps an infrastructure-specific runtime error.
    #[must_use]
    pub fn infrastructure(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Infrastructure(Arc::new(err))
    }
}
