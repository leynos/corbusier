//! In-memory runtime adapter for orchestrated agent turns.

use crate::agent_backend::{
    domain::{
        AgentBackendRegistration, RuntimeSessionId, TurnExecutionRequest, TurnExecutionResult,
    },
    ports::{AgentRuntimeError, AgentRuntimePort, AgentRuntimeResult},
};
use async_trait::async_trait;
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use uuid::Uuid;

/// Recorded runtime execution request for assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExecutionRecord {
    /// Backend ID used for execution.
    pub backend_id: crate::agent_backend::domain::BackendId,
    /// Runtime session ID used for execution.
    pub runtime_session_id: RuntimeSessionId,
    /// Request payload sent to runtime.
    pub request: TurnExecutionRequest,
    /// Instant when runtime execution started.
    pub started_at: Instant,
    /// Instant when runtime execution completed.
    pub completed_at: Instant,
}

#[derive(Debug, Default)]
struct InMemoryRuntimeState {
    next_session_ordinal: u64,
    fail_session_creation_for: HashSet<crate::agent_backend::domain::BackendId>,
    queued_results: VecDeque<TurnExecutionResult>,
    queued_execute_delays: VecDeque<std::time::Duration>,
    next_execute_failure: Option<String>,
    created_session_ids: Vec<RuntimeSessionId>,
    execution_records: Vec<RuntimeExecutionRecord>,
}

/// Thread-safe in-memory runtime adapter.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAgentRuntime {
    state: Arc<RwLock<InMemoryRuntimeState>>,
}

impl InMemoryAgentRuntime {
    /// Creates a new in-memory runtime with empty state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one turn result for the next `execute_turn` call.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn queue_turn_result(&self, result: TurnExecutionResult) -> AgentRuntimeResult<()> {
        let mut state = self.state.write().map_err(|err| {
            AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
        state.queued_results.push_back(result);
        Ok(())
    }

    /// Queues one runtime execution delay for the next `execute_turn` call.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn queue_execute_delay(&self, delay: std::time::Duration) -> AgentRuntimeResult<()> {
        let mut state = self.state.write().map_err(|err| {
            AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
        state.queued_execute_delays.push_back(delay);
        Ok(())
    }

    /// Configures one execute-turn call to fail.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn fail_next_execute(&self, message: impl Into<String>) -> AgentRuntimeResult<()> {
        let mut state = self.state.write().map_err(|err| {
            AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
        state.next_execute_failure = Some(message.into());
        Ok(())
    }

    /// Configures session creation to fail for the given backend ID.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn fail_session_creation_for(
        &self,
        backend_id: crate::agent_backend::domain::BackendId,
    ) -> AgentRuntimeResult<()> {
        let mut state = self.state.write().map_err(|err| {
            AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
        state.fail_session_creation_for.insert(backend_id);
        Ok(())
    }

    /// Returns created runtime session IDs.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn created_session_ids(&self) -> AgentRuntimeResult<Vec<RuntimeSessionId>> {
        let state = self.state.read().map_err(|err| {
            AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
        Ok(state.created_session_ids.clone())
    }

    /// Returns recorded runtime execution requests.
    ///
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn execution_records(&self) -> AgentRuntimeResult<Vec<RuntimeExecutionRecord>> {
        let state = self.state.read().map_err(|err| {
            AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
        Ok(state.execution_records.clone())
    }
}

#[async_trait]
impl AgentRuntimePort for InMemoryAgentRuntime {
    async fn create_session(
        &self,
        backend: &AgentBackendRegistration,
        _conversation_id: Uuid,
    ) -> AgentRuntimeResult<RuntimeSessionId> {
        let mut state = self.state.write().map_err(|err| {
            AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
        })?;

        if state.fail_session_creation_for.contains(&backend.id()) {
            return Err(AgentRuntimeError::SessionCreationFailed(format!(
                "configured failure for backend {}",
                backend.id()
            )));
        }

        state.next_session_ordinal = state.next_session_ordinal.saturating_add(1);
        let session_id = format!(
            "{}-session-{}",
            backend.name().as_str(),
            state.next_session_ordinal
        );
        let parsed_session_id = RuntimeSessionId::new(session_id)
            .map_err(|_| AgentRuntimeError::InvalidRuntimeSessionId)?;
        state.created_session_ids.push(parsed_session_id.clone());
        Ok(parsed_session_id)
    }

    async fn teardown_session(
        &self,
        _backend: &AgentBackendRegistration,
        runtime_session_id: &RuntimeSessionId,
    ) -> AgentRuntimeResult<()> {
        let mut state = self
            .state
            .write()
            .map_err(|err| AgentRuntimeError::SessionTeardownFailed(err.to_string()))?;
        state
            .created_session_ids
            .retain(|id| id != runtime_session_id);
        Ok(())
    }

    async fn execute_turn(
        &self,
        backend: &AgentBackendRegistration,
        runtime_session_id: &RuntimeSessionId,
        request: &TurnExecutionRequest,
    ) -> AgentRuntimeResult<TurnExecutionResult> {
        let (delay, execute_failure, result) = {
            let mut state = self.state.write().map_err(|err| {
                AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
            })?;

            let delay = state.queued_execute_delays.pop_front();
            let execute_failure = state.next_execute_failure.take();
            let result = state.queued_results.pop_front().unwrap_or_else(|| {
                TurnExecutionResult::new("in-memory default response", Vec::new())
            });
            (delay, execute_failure, result)
        };

        let started_at = Instant::now();
        if let Some(execution_delay) = delay {
            tokio::time::sleep(execution_delay).await;
        }

        if let Some(message) = execute_failure {
            return Err(AgentRuntimeError::TurnExecutionFailed(message));
        }

        let completed_at = Instant::now();
        let mut state = self.state.write().map_err(|err| {
            AgentRuntimeError::infrastructure(std::io::Error::other(err.to_string()))
        })?;

        state.execution_records.push(RuntimeExecutionRecord {
            backend_id: backend.id(),
            runtime_session_id: runtime_session_id.clone(),
            request: request.clone(),
            started_at,
            completed_at,
        });

        Ok(result)
    }
}
