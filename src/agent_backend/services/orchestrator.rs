//! Service layer for agent turn orchestration and session continuity.

use crate::agent_backend::{
    domain::{
        AgentBackendRegistration, BackendId, BackendStatus, ToolCallAudit, ToolCallAuditStatus,
        ToolCallRequest, ToolCallResult, TurnExecutionRequest, TurnSession,
        TurnSessionCreateParams, TurnSessionDomainError, TurnSessionId, deterministic_tool_call_id,
    },
    ports::{
        AgentRuntimeError, AgentRuntimePort, BackendRegistryError, BackendRegistryRepository,
        SessionSlotArbitration, ToolRouterPort, ToolRoutingContext, ToolRoutingError,
        TurnSessionRepository, TurnSessionRepositoryError,
    },
};
use crate::context::RequestContext;
use chrono::Duration;
use mockable::Clock;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use thiserror::Error;
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;

type SessionKey = (BackendId, Uuid);
type SessionLock = Arc<Mutex<()>>;
type SessionLockRef = Weak<Mutex<()>>;

#[derive(Debug)]
struct SessionExecutionLocks {
    locks: std::sync::Mutex<HashMap<SessionKey, SessionLockRef>>,
}

impl SessionExecutionLocks {
    fn new() -> Self {
        Self {
            locks: std::sync::Mutex::new(HashMap::new()),
        }
    }

    fn lock_for(&self, backend_id: BackendId, conversation_id: Uuid) -> SessionLock {
        let key = (backend_id, conversation_id);
        let mut locks = match self.locks.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(existing) = locks.get(&key).and_then(Weak::upgrade) {
            return existing;
        }

        let created = Arc::new(Mutex::new(()));
        locks.insert(key, Arc::downgrade(&created));
        created
    }

    async fn lock(&self, backend_id: BackendId, conversation_id: Uuid) -> OwnedMutexGuard<()> {
        self.lock_for(backend_id, conversation_id)
            .lock_owned()
            .await
    }
}

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

impl ExecuteAgentTurnResponse {
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

/// Dependency bundle for [`AgentTurnOrchestratorService`].
pub struct AgentTurnOrchestratorPorts<R, S, RT, TR, C>
where
    R: BackendRegistryRepository,
    S: TurnSessionRepository,
    RT: AgentRuntimePort,
    TR: ToolRouterPort,
    C: Clock + Send + Sync,
{
    /// Backend registration repository port.
    pub backend_registry: Arc<R>,
    /// Turn-session persistence repository port.
    pub turn_sessions: Arc<S>,
    /// Backend runtime execution port.
    pub runtime: Arc<RT>,
    /// Tool routing port.
    pub tool_router: Arc<TR>,
    /// Clock dependency for deterministic session policy evaluation.
    pub clock: Arc<C>,
}

/// Service that orchestrates backend turns and session lifecycle.
#[derive(Clone)]
pub struct AgentTurnOrchestratorService<R, S, RT, TR, C>
where
    R: BackendRegistryRepository,
    S: TurnSessionRepository,
    RT: AgentRuntimePort,
    TR: ToolRouterPort,
    C: Clock + Send + Sync,
{
    backend_registry: Arc<R>,
    turn_sessions: Arc<S>,
    runtime: Arc<RT>,
    tool_router: Arc<TR>,
    clock: Arc<C>,
    config: AgentTurnOrchestratorConfig,
    execution_locks: Arc<SessionExecutionLocks>,
}

impl<R, S, RT, TR, C> AgentTurnOrchestratorService<R, S, RT, TR, C>
where
    R: BackendRegistryRepository,
    S: TurnSessionRepository,
    RT: AgentRuntimePort,
    TR: ToolRouterPort,
    C: Clock + Send + Sync,
{
    /// Creates an orchestration service with explicit configuration.
    #[must_use]
    pub fn with_config(
        ports: AgentTurnOrchestratorPorts<R, S, RT, TR, C>,
        config: AgentTurnOrchestratorConfig,
    ) -> Self {
        Self {
            backend_registry: ports.backend_registry,
            turn_sessions: ports.turn_sessions,
            runtime: ports.runtime,
            tool_router: ports.tool_router,
            clock: ports.clock,
            config,
            execution_locks: Arc::new(SessionExecutionLocks::new()),
        }
    }

    /// Creates an orchestration service with default configuration.
    #[must_use]
    pub fn new(ports: AgentTurnOrchestratorPorts<R, S, RT, TR, C>) -> Self {
        Self::with_config(ports, AgentTurnOrchestratorConfig::default())
    }

    /// Executes one agent turn with deterministic tool routing.
    ///
    /// # Errors
    ///
    /// Returns [`AgentTurnOrchestrationError`] when backend lookup fails,
    /// session lifecycle operations fail, runtime execution fails, or tool
    /// routing fails.
    pub async fn execute_turn(
        &self,
        ctx: &RequestContext,
        request: ExecuteAgentTurnRequest,
    ) -> AgentTurnOrchestrationResult<ExecuteAgentTurnResponse> {
        let conversation_id = request.turn.conversation_id();
        let _execution_guard = self
            .execution_locks
            .lock(request.backend_id, conversation_id)
            .await;

        let backend = self
            .backend_registry
            .find_by_id(ctx, request.backend_id)
            .await?
            .ok_or(AgentTurnOrchestrationError::BackendNotFound(
                request.backend_id,
            ))?;

        if backend.status() != BackendStatus::Active {
            return Err(AgentTurnOrchestrationError::BackendInactive(backend.id()));
        }

        let session_resolution_now = self.clock.utc();

        let (mut session, reused_session, rotated_session) = self
            .resolve_session(&backend, conversation_id, session_resolution_now)
            .await?;

        let runtime_result = self
            .runtime
            .execute_turn(&backend, session.runtime_session_handle(), &request.turn)
            .await?;

        let (tool_results, tool_call_audits) = self
            .route_tool_calls(&session, runtime_result.tool_calls())
            .await?;

        let completion_time = self.clock.utc();
        session.record_turn(completion_time)?;
        self.turn_sessions.upsert_session(&session).await?;

        Ok(ExecuteAgentTurnResponse {
            session_id: session.id(),
            runtime_session_id: session.runtime_session_id().to_owned(),
            assistant_response: runtime_result.assistant_response().to_owned(),
            tool_results,
            tool_call_audits,
            reused_session,
            rotated_session,
        })
    }

    async fn resolve_session(
        &self,
        backend: &AgentBackendRegistration,
        conversation_id: Uuid,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AgentTurnOrchestrationResult<(TurnSession, bool, bool)> {
        match self
            .turn_sessions
            .arbitrate_session_slot(backend.id(), conversation_id, now)
            .await?
        {
            SessionSlotArbitration::Reused(existing) => Ok((existing, true, false)),
            SessionSlotArbitration::Vacant => {
                let (created_session, reused_due_to_conflict) = self
                    .create_or_reuse_session(backend, conversation_id, now)
                    .await?;
                Ok((created_session, reused_due_to_conflict, false))
            }
            SessionSlotArbitration::Expired => {
                let (rotated_session, reused_due_to_conflict) = self
                    .create_or_reuse_session(backend, conversation_id, now)
                    .await?;
                Ok((rotated_session, reused_due_to_conflict, true))
            }
        }
    }

    async fn create_session(
        &self,
        backend: &AgentBackendRegistration,
        conversation_id: Uuid,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AgentTurnOrchestrationResult<TurnSession> {
        let runtime_session_id = self
            .runtime
            .create_session(backend, conversation_id)
            .await?;

        let session = match TurnSession::new(TurnSessionCreateParams {
            backend_id: backend.id(),
            conversation_id,
            runtime_session_id: runtime_session_id.clone(),
            ttl: self.config.session_ttl(),
            now,
        }) {
            Ok(session) => session,
            Err(error) => {
                self.runtime
                    .teardown_session(backend, &runtime_session_id)
                    .await?;
                return Err(AgentTurnOrchestrationError::SessionDomain(error));
            }
        };

        match self.turn_sessions.upsert_session(&session).await {
            Ok(()) => Ok(session),
            Err(error) => {
                self.runtime
                    .teardown_session(backend, &runtime_session_id)
                    .await?;
                Err(AgentTurnOrchestrationError::SessionRepository(error))
            }
        }
    }

    async fn create_or_reuse_session(
        &self,
        backend: &AgentBackendRegistration,
        conversation_id: Uuid,
        now: chrono::DateTime<chrono::Utc>,
    ) -> AgentTurnOrchestrationResult<(TurnSession, bool)> {
        match self.create_session(backend, conversation_id, now).await {
            Ok(created) => Ok((created, false)),
            Err(AgentTurnOrchestrationError::SessionRepository(
                TurnSessionRepositoryError::ActiveSessionConflict { .. },
            )) => {
                let active = self
                    .require_active_session_after_conflict(backend.id(), conversation_id)
                    .await?;
                Ok((active, true))
            }
            Err(other) => Err(other),
        }
    }

    async fn require_active_session_after_conflict(
        &self,
        backend_id: BackendId,
        conversation_id: Uuid,
    ) -> AgentTurnOrchestrationResult<TurnSession> {
        self.turn_sessions
            .find_active_session(backend_id, conversation_id)
            .await?
            .ok_or(AgentTurnOrchestrationError::SessionRepository(
                TurnSessionRepositoryError::active_session_conflict(backend_id, conversation_id),
            ))
    }

    async fn route_tool_calls(
        &self,
        session: &TurnSession,
        tool_calls: &[ToolCallRequest],
    ) -> AgentTurnOrchestrationResult<(Vec<ToolCallResult>, Vec<ToolCallAudit>)> {
        let mut tool_results = Vec::with_capacity(tool_calls.len());
        let mut audits = Vec::with_capacity(tool_calls.len());

        for (index, tool_call) in tool_calls.iter().enumerate() {
            let call_id = deterministic_tool_call_id(tool_call, index);
            let context = ToolRoutingContext::new(
                session.backend_id(),
                session.conversation_id(),
                session.id(),
            );
            match self
                .tool_router
                .route_tool_call(&call_id, tool_call, context)
                .await
            {
                Ok(result) => {
                    tool_results.push(result);
                    audits.push(ToolCallAudit::new(
                        call_id,
                        tool_call.tool_name(),
                        ToolCallAuditStatus::Succeeded,
                    ));
                }
                Err(source) => {
                    return Err(AgentTurnOrchestrationError::ToolRouting {
                        call_id,
                        tool_name: tool_call.tool_name().to_owned(),
                        source,
                    });
                }
            }
        }

        Ok((tool_results, audits))
    }
}
