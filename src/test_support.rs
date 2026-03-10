//! Shared test utilities available to all `#[cfg(test)]` modules.

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
use std::collections::HashSet;

/// Creates a [`RequestContext`] with freshly generated identifiers.
pub fn test_request_ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
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
    fn with_started_lock<F, T>(&self, operation: F) -> McpServerHostResult<T>
    where
        F: FnOnce(&mut HashSet<McpServerId>) -> T,
    {
        let mut started =
            self.started
                .lock()
                .map_err(|err| McpServerHostError::CommunicationError {
                    server_id: McpServerId::from_uuid(uuid::Uuid::nil()),
                    reason: err.to_string(),
                })?;
        Ok(operation(&mut started))
    }

    fn modify_started(&self, server_id: McpServerId, insert: bool) -> McpServerHostResult<()> {
        self.with_started_lock(|started| {
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
        self.with_started_lock(|started| {
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
        self.with_started_lock(|started| {
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
