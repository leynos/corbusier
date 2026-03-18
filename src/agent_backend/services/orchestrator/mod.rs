//! Service layer for agent turn orchestration and session continuity.

mod errors;
mod execution_locks;
mod types;

pub use errors::{AgentTurnOrchestrationError, AgentTurnOrchestrationResult};
use execution_locks::SessionExecutionLocks;
use types::ExecuteAgentTurnResponseParts;
pub use types::{AgentTurnOrchestratorConfig, ExecuteAgentTurnRequest, ExecuteAgentTurnResponse};

use crate::agent_backend::{
    domain::{
        AgentBackendRegistration, BackendId, BackendStatus, ToolCallAudit, ToolCallAuditStatus,
        ToolCallRequest, ToolCallResult, TurnSession, TurnSessionCreateParams,
        deterministic_tool_call_id,
    },
    ports::{
        AgentRuntimePort, BackendRegistryRepository, SessionSlotArbitration, SessionSlotKey,
        ToolRouterPort, ToolRoutingContext, TurnSessionRepository, TurnSessionRepositoryError,
    },
};
use crate::context::RequestContext;
use chrono::Utc;
use mockable::Clock;
use std::sync::Arc;
use uuid::Uuid;

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

struct SessionResolutionParams<'a> {
    ctx: &'a RequestContext,
    backend: &'a AgentBackendRegistration,
    conversation_id: Uuid,
    now: chrono::DateTime<Utc>,
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

        let resolution_params = SessionResolutionParams {
            ctx,
            backend: &backend,
            conversation_id,
            now: self.clock.utc(),
        };
        let (mut session, reused_session, rotated_session) =
            self.resolve_session(&resolution_params).await?;

        let runtime_result = match self
            .runtime
            .execute_turn(&backend, session.runtime_session_handle(), &request.turn)
            .await
        {
            Ok(result) => result,
            Err(error) => {
                let expire_time = self.clock.utc();
                session.mark_expired(expire_time);
                drop(self.turn_sessions.upsert_session(ctx, &session).await);
                drop(
                    self.runtime
                        .teardown_session(&backend, session.runtime_session_handle())
                        .await,
                );
                return Err(error.into());
            }
        };

        let (tool_results, tool_call_audits) = self
            .route_tool_calls(&session, runtime_result.tool_calls())
            .await?;

        let completion_time = self.clock.utc();
        session.record_turn(completion_time)?;
        self.turn_sessions.upsert_session(ctx, &session).await?;

        Ok(ExecuteAgentTurnResponse::new(
            &session,
            ExecuteAgentTurnResponseParts {
                assistant_response: runtime_result.assistant_response().to_owned(),
                tool_results,
                tool_call_audits,
                reused_session,
                rotated_session,
            },
        ))
    }

    async fn resolve_session(
        &self,
        params: &SessionResolutionParams<'_>,
    ) -> AgentTurnOrchestrationResult<(TurnSession, bool, bool)> {
        match self
            .turn_sessions
            .arbitrate_session_slot(
                params.ctx,
                SessionSlotKey::new(params.backend.id(), params.conversation_id),
                params.now,
            )
            .await?
        {
            SessionSlotArbitration::Reused(existing) => Ok((existing, true, false)),
            SessionSlotArbitration::Vacant => {
                let (created_session, reused_due_to_conflict) =
                    self.create_or_reuse_session(params).await?;
                Ok((created_session, reused_due_to_conflict, false))
            }
            SessionSlotArbitration::Expired => {
                let (rotated_session, reused_due_to_conflict) =
                    self.create_or_reuse_session(params).await?;
                Ok((rotated_session, reused_due_to_conflict, true))
            }
        }
    }

    async fn create_session(
        &self,
        params: &SessionResolutionParams<'_>,
    ) -> AgentTurnOrchestrationResult<TurnSession> {
        let runtime_session_id = self
            .runtime
            .create_session(params.backend, params.conversation_id)
            .await?;

        let now = self.clock.utc();
        let session = match TurnSession::new(TurnSessionCreateParams {
            backend_id: params.backend.id(),
            conversation_id: params.conversation_id,
            runtime_session_id: runtime_session_id.clone(),
            ttl: self.config.session_ttl(),
            now,
        }) {
            Ok(session) => session,
            Err(error) => {
                self.runtime
                    .teardown_session(params.backend, &runtime_session_id)
                    .await?;
                return Err(AgentTurnOrchestrationError::SessionDomain(error));
            }
        };

        match self
            .turn_sessions
            .upsert_session(params.ctx, &session)
            .await
        {
            Ok(()) => Ok(session),
            Err(error) => {
                self.runtime
                    .teardown_session(params.backend, &runtime_session_id)
                    .await?;
                Err(AgentTurnOrchestrationError::SessionRepository(error))
            }
        }
    }

    async fn create_or_reuse_session(
        &self,
        params: &SessionResolutionParams<'_>,
    ) -> AgentTurnOrchestrationResult<(TurnSession, bool)> {
        match self.create_session(params).await {
            Ok(created) => Ok((created, false)),
            Err(AgentTurnOrchestrationError::SessionRepository(
                TurnSessionRepositoryError::ActiveSessionConflict { .. },
            )) => {
                let active = self
                    .require_active_session_after_conflict(
                        params.ctx,
                        params.backend.id(),
                        params.conversation_id,
                    )
                    .await?;
                Ok((active, true))
            }
            Err(other) => Err(other),
        }
    }

    async fn require_active_session_after_conflict(
        &self,
        ctx: &RequestContext,
        backend_id: BackendId,
        conversation_id: Uuid,
    ) -> AgentTurnOrchestrationResult<TurnSession> {
        self.turn_sessions
            .find_active_session(ctx, SessionSlotKey::new(backend_id, conversation_id))
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
