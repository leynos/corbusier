//! Shared test utilities available to all `#[cfg(test)]` modules.

use crate::agent_backend::{
    adapters::memory::{
        InMemoryAgentRuntime, InMemoryBackendRegistry, InMemoryToolRouter,
        InMemoryTurnSessionRepository,
    },
    services::{
        AgentTurnOrchestratorConfig, AgentTurnOrchestratorPorts, AgentTurnOrchestratorService,
    },
};
use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use crate::tool_registry::{
    domain::{
        McpServerHealthSnapshot, McpServerId, McpServerRegistration, McpToolDefinition,
        ToolCallRequest,
    },
    ports::{
        McpServerHost, McpServerHostError, McpServerHostResult, StartHostResult, ToolCallHostResult,
    },
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use std::collections::HashSet;
use std::sync::Arc;

/// Shared in-memory orchestrator type for agent-turn tests.
pub type InMemoryAgentTurnOrchestrator = AgentTurnOrchestratorService<
    InMemoryBackendRegistry,
    InMemoryTurnSessionRepository,
    InMemoryAgentRuntime,
    InMemoryToolRouter,
    mockable::DefaultClock,
>;

/// Fully wired in-memory stack for agent-turn orchestration tests.
pub struct InMemoryAgentTurnStack {
    /// Backend registry adapter.
    pub backend_registry: Arc<InMemoryBackendRegistry>,
    /// Turn-session repository adapter.
    pub session_repository: Arc<InMemoryTurnSessionRepository>,
    /// Runtime adapter.
    pub runtime: Arc<InMemoryAgentRuntime>,
    /// Tool-router adapter.
    pub tool_router: Arc<InMemoryToolRouter>,
    /// Shared clock dependency.
    pub clock: Arc<mockable::DefaultClock>,
    /// Default request context.
    pub ctx: RequestContext,
    /// Orchestrator service under test.
    pub service: InMemoryAgentTurnOrchestrator,
}

/// Creates a [`RequestContext`] with freshly generated identifiers.
#[must_use]
pub fn test_request_ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}

/// Clones a request context into a different tenant while preserving the
/// correlation, user, and session identifiers.
#[must_use]
pub fn other_tenant_ctx(source: &RequestContext) -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        source.correlation_id(),
        source.user_id(),
        source.session_id(),
    )
}

/// Builds the default in-memory stack used by agent-turn tests.
#[must_use]
pub fn build_in_memory_orchestrator() -> InMemoryAgentTurnStack {
    let backend_registry = Arc::new(InMemoryBackendRegistry::new());
    let session_repository = Arc::new(InMemoryTurnSessionRepository::new());
    let runtime = Arc::new(InMemoryAgentRuntime::new());
    let tool_router = Arc::new(InMemoryToolRouter::new());
    let clock = Arc::new(mockable::DefaultClock);
    let service = AgentTurnOrchestratorService::with_config(
        AgentTurnOrchestratorPorts {
            backend_registry: backend_registry.clone(),
            turn_sessions: session_repository.clone(),
            runtime: runtime.clone(),
            tool_router: tool_router.clone(),
            clock: clock.clone(),
        },
        AgentTurnOrchestratorConfig::default(),
    );

    InMemoryAgentTurnStack {
        backend_registry,
        session_repository,
        runtime,
        tool_router,
        clock,
        ctx: test_request_ctx(),
        service,
    }
}
/// Inserts a placeholder tenant row if one does not already exist.
///
/// Delegates to the canonical [`bootstrap_tenant_row`] in `tenant_tx`,
/// exposed here so integration tests can seed tenant foreign-key targets
/// without duplicating the SQL.
///
/// # Errors
///
/// Returns a Diesel error if the insert query fails.
///
/// # Examples
///
/// ```ignore
/// bootstrap_tenant_row(&mut conn, tenant_uuid)?;
/// ```
pub fn bootstrap_tenant_row(
    conn: &mut PgConnection,
    tenant_id: TenantId,
) -> diesel::QueryResult<usize> {
    crate::message::adapters::postgres::tenant_tx::bootstrap_tenant_row(
        conn,
        tenant_id.into_inner(),
    )
}

/// Fake MCP host adapter whose [`McpServerHost::health`] always
/// returns an error, simulating an unreachable health probe.
#[derive(Debug, Clone, Default)]
pub struct HealthProbeFailureHost {
    started: std::sync::Arc<std::sync::Mutex<HashSet<McpServerId>>>,
}

impl HealthProbeFailureHost {
    /// Acquires the lock on started servers and applies `operation`.
    fn with_started_lock<F, T>(
        &self,
        server_id: McpServerId,
        operation: F,
    ) -> McpServerHostResult<T>
    where
        F: FnOnce(&mut HashSet<McpServerId>) -> T,
    {
        let mut started =
            self.started
                .lock()
                .map_err(|err| McpServerHostError::CommunicationError {
                    server_id,
                    reason: err.to_string(),
                })?;
        Ok(operation(&mut started))
    }

    fn modify_started(&self, server_id: McpServerId, insert: bool) -> McpServerHostResult<()> {
        self.with_started_lock(server_id, |started| {
            if insert {
                started.insert(server_id);
            } else {
                started.remove(&server_id);
            }
        })
    }
}

#[async_trait]
impl McpServerHost for HealthProbeFailureHost {
    async fn start(
        &self,
        _ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<StartHostResult> {
        self.modify_started(server.id(), true)?;
        Ok(StartHostResult::default())
    }

    async fn stop(
        &self,
        _ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<()> {
        self.modify_started(server.id(), false)
    }

    async fn health(
        &self,
        _ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<McpServerHealthSnapshot> {
        Err(McpServerHostError::CommunicationError {
            server_id: server.id(),
            reason: "health probe unavailable".to_owned(),
        })
    }

    async fn list_tools(
        &self,
        _ctx: &RequestContext,
        server: &McpServerRegistration,
    ) -> McpServerHostResult<Vec<McpToolDefinition>> {
        self.with_started_lock(server.id(), |started| {
            if started.contains(&server.id()) {
                Ok(vec![])
            } else {
                Err(McpServerHostError::NotRunning(server.id()))
            }
        })?
    }

    async fn call_tool(
        &self,
        _ctx: &RequestContext,
        server: &McpServerRegistration,
        _request: &ToolCallRequest,
    ) -> McpServerHostResult<ToolCallHostResult> {
        self.with_started_lock(server.id(), |started| {
            if started.contains(&server.id()) {
                Ok(ToolCallHostResult {
                    content: serde_json::Value::Null,
                    stderr_output: None,
                })
            } else {
                Err(McpServerHostError::NotRunning(server.id()))
            }
        })?
    }
}
