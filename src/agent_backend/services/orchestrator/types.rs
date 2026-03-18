//! Request, response, and configuration types for turn orchestration.

use super::AgentTurnOrchestrationError;
use crate::agent_backend::domain::{
    BackendId, ToolCallAudit, ToolCallResult, TurnExecutionRequest, TurnSession, TurnSessionId,
};
use chrono::Duration;

/// Configuration for turn orchestration behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentTurnOrchestratorConfig {
    session_ttl: Duration,
}

impl AgentTurnOrchestratorConfig {
    /// Creates orchestration configuration from a session TTL duration.
    ///
    /// # Errors
    ///
    /// Returns [`AgentTurnOrchestrationError::InvalidSessionTtl`] when the
    /// duration is not strictly positive.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Constructor intentionally remains non-const to avoid committing to const API semantics."
    )]
    pub fn new(session_ttl: Duration) -> Result<Self, AgentTurnOrchestrationError> {
        let ttl_seconds = session_ttl.num_seconds();
        if ttl_seconds <= 0 {
            return Err(AgentTurnOrchestrationError::InvalidSessionTtl(ttl_seconds));
        }
        Ok(Self { session_ttl })
    }

    /// Returns configured session TTL.
    #[must_use]
    pub const fn session_ttl(self) -> Duration {
        self.session_ttl
    }
}

impl Default for AgentTurnOrchestratorConfig {
    fn default() -> Self {
        Self {
            session_ttl: Duration::minutes(30),
        }
    }
}

/// Request payload for executing an orchestrated agent turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteAgentTurnRequest {
    /// Backend registration identifier.
    pub backend_id: BackendId,
    /// Canonical turn request payload.
    pub turn: TurnExecutionRequest,
}

impl ExecuteAgentTurnRequest {
    /// Creates an execute-turn request.
    #[must_use]
    pub const fn new(backend_id: BackendId, turn: TurnExecutionRequest) -> Self {
        Self { backend_id, turn }
    }
}

/// Orchestrated turn response with routed tool details and session metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteAgentTurnResponse {
    session_id: TurnSessionId,
    runtime_session_id: String,
    assistant_response: String,
    tool_results: Vec<ToolCallResult>,
    tool_call_audits: Vec<ToolCallAudit>,
    reused_session: bool,
    rotated_session: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecuteAgentTurnResponseParts {
    pub(super) assistant_response: String,
    pub(super) tool_results: Vec<ToolCallResult>,
    pub(super) tool_call_audits: Vec<ToolCallAudit>,
    pub(super) reused_session: bool,
    pub(super) rotated_session: bool,
}

impl ExecuteAgentTurnResponse {
    #[must_use]
    pub(super) fn new(session: &TurnSession, parts: ExecuteAgentTurnResponseParts) -> Self {
        Self {
            session_id: session.id(),
            runtime_session_id: session.runtime_session_id().to_owned(),
            assistant_response: parts.assistant_response,
            tool_results: parts.tool_results,
            tool_call_audits: parts.tool_call_audits,
            reused_session: parts.reused_session,
            rotated_session: parts.rotated_session,
        }
    }

    /// Returns orchestration session ID.
    #[must_use]
    pub const fn session_id(&self) -> TurnSessionId {
        self.session_id
    }

    /// Returns backend-native runtime session ID.
    #[must_use]
    pub fn runtime_session_id(&self) -> &str {
        &self.runtime_session_id
    }

    /// Returns assistant response text.
    #[must_use]
    pub fn assistant_response(&self) -> &str {
        &self.assistant_response
    }

    /// Returns routed tool results.
    #[must_use]
    pub fn tool_results(&self) -> &[ToolCallResult] {
        &self.tool_results
    }

    /// Returns tool call audits emitted by orchestration.
    #[must_use]
    pub fn tool_call_audits(&self) -> &[ToolCallAudit] {
        &self.tool_call_audits
    }

    /// Returns `true` when an existing active session was reused.
    #[must_use]
    pub const fn reused_session(&self) -> bool {
        self.reused_session
    }

    /// Returns `true` when an expired session was rotated.
    #[must_use]
    pub const fn rotated_session(&self) -> bool {
        self.rotated_session
    }
}
