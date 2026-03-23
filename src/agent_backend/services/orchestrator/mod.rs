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
        AgentBackendRegistration, BackendStatus, ToolCallAudit, ToolCallAuditStatus,
        ToolCallRequest, ToolCallResult, TurnSession, deterministic_tool_call_id,
    },
    ports::{
        AgentRuntimePort, BackendRegistryRepository, SessionSlotArbitration, SessionSlotKey,
        SessionSlotReservation, ToolRouterPort, ToolRoutingContext, TurnSessionRepository,
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
            .lock(ctx.tenant_id(), conversation_id)
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
                self.expire_persist_and_teardown(ctx, &backend, &mut session)
                    .await?;
                return Err(error.into());
            }
        };

        let (tool_results, tool_call_audits) = match self
            .route_tool_calls(ctx.tenant_id(), &session, runtime_result.tool_calls())
            .await
        {
            Ok(routed) => routed,
            Err(error) => {
                self.expire_persist_and_teardown(ctx, &backend, &mut session)
                    .await?;
                return Err(error);
            }
        };

        self.persist_completed_turn(ctx, &backend, &mut session, reused_session)
            .await?;

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
                SessionSlotReservation::new(
                    SessionSlotKey::new(params.backend.id(), params.conversation_id),
                    params.now,
                    self.config.session_ttl(),
                ),
            )
            .await?
        {
            SessionSlotArbitration::Reused(existing) => Ok((existing, true, false)),
            SessionSlotArbitration::Reserved {
                reservation,
                prior_expired,
            } => {
                let rotated_session = prior_expired.is_some();
                let activated = self.activate_reserved_session(params, reservation).await?;
                if let Some(expired_session) = prior_expired {
                    self.runtime
                        .teardown_session(params.backend, expired_session.runtime_session_handle())
                        .await?;
                }
                Ok((activated, false, rotated_session))
            }
        }
    }

    async fn expire_persist_and_teardown(
        &self,
        ctx: &RequestContext,
        backend: &AgentBackendRegistration,
        session: &mut TurnSession,
    ) -> AgentTurnOrchestrationResult<()> {
        let expire_time = self.clock.utc();
        session.mark_expired(expire_time);

        self.turn_sessions
            .upsert_session(ctx, session)
            .await
            .map_err(AgentTurnOrchestrationError::SessionRepository)?;
        self.runtime
            .teardown_session(backend, session.runtime_session_handle())
            .await
            .map_err(AgentTurnOrchestrationError::Runtime)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "all parameters are necessary for session persistence and cleanup"
    )]
    async fn persist_completed_turn(
        &self,
        ctx: &RequestContext,
        backend: &AgentBackendRegistration,
        session: &mut TurnSession,
        reused_session: bool,
    ) -> AgentTurnOrchestrationResult<()> {
        let completion_time = self.clock.utc();
        session.record_turn(completion_time)?;

        self.persist_session_or_cleanup(ctx, backend, session, reused_session)
            .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "all parameters are necessary for session persistence and cleanup"
    )]
    async fn persist_session_or_cleanup(
        &self,
        ctx: &RequestContext,
        backend: &AgentBackendRegistration,
        session: &mut TurnSession,
        reused_session: bool,
    ) -> AgentTurnOrchestrationResult<()> {
        if let Err(error) = self.turn_sessions.upsert_session(ctx, session).await {
            self.cleanup_after_upsert_failure(ctx, backend, session, reused_session)
                .await;
            return Err(AgentTurnOrchestrationError::SessionRepository(error));
        }
        Ok(())
    }

    #[expect(
        clippy::cognitive_complexity,
        reason = "nested error handling is necessary for proper cleanup logging"
    )]
    #[expect(
        clippy::too_many_arguments,
        reason = "all parameters are necessary for conditional cleanup logic"
    )]
    async fn cleanup_after_upsert_failure(
        &self,
        ctx: &RequestContext,
        backend: &AgentBackendRegistration,
        session: &mut TurnSession,
        reused_session: bool,
    ) {
        if !reused_session
            && let Err(cleanup_err) = self
                .expire_persist_and_teardown(ctx, backend, session)
                .await
        {
            tracing::warn!(
                error = ?cleanup_err,
                backend_id = %session.backend_id(),
                "cleanup failed after session upsert failure; session may leak"
            );
        }
    }

    async fn expire_session(
        &self,
        ctx: &RequestContext,
        mut reservation: TurnSession,
    ) -> AgentTurnOrchestrationResult<()> {
        reservation.mark_expired(self.clock.utc());
        self.turn_sessions.upsert_session(ctx, &reservation).await?;
        Ok(())
    }

    async fn activate_reserved_session(
        &self,
        params: &SessionResolutionParams<'_>,
        mut reservation: TurnSession,
    ) -> AgentTurnOrchestrationResult<TurnSession> {
        let runtime_session_id = match self
            .runtime
            .create_session(params.backend, params.conversation_id)
            .await
        {
            Ok(runtime_session_id) => runtime_session_id,
            Err(error) => {
                self.expire_session(params.ctx, reservation).await?;
                return Err(error.into());
            }
        };

        if let Err(error) = reservation.activate(runtime_session_id.clone()) {
            self.runtime
                .teardown_session(params.backend, &runtime_session_id)
                .await?;
            self.expire_session(params.ctx, reservation).await?;
            return Err(AgentTurnOrchestrationError::SessionDomain(error));
        }

        if let Err(error) = self
            .turn_sessions
            .upsert_session(params.ctx, &reservation)
            .await
        {
            self.runtime
                .teardown_session(params.backend, &runtime_session_id)
                .await?;
            self.expire_session(params.ctx, reservation.clone()).await?;
            return Err(AgentTurnOrchestrationError::SessionRepository(error));
        }

        Ok(reservation)
    }

    async fn route_tool_calls(
        &self,
        tenant_id: crate::context::TenantId,
        session: &TurnSession,
        tool_calls: &[ToolCallRequest],
    ) -> AgentTurnOrchestrationResult<(Vec<ToolCallResult>, Vec<ToolCallAudit>)> {
        let mut tool_results = Vec::with_capacity(tool_calls.len());
        let mut audits = Vec::with_capacity(tool_calls.len());

        for (index, tool_call) in tool_calls.iter().enumerate() {
            let call_id = deterministic_tool_call_id(tool_call, index);
            let context = ToolRoutingContext::new(
                tenant_id,
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
