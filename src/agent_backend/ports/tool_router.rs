//! Tool routing port for orchestrated agent turns.

use crate::agent_backend::domain::{BackendId, ToolCallRequest, ToolCallResult, TurnSessionId};
use crate::context::TenantId;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Result type for tool-routing operations.
pub type ToolRoutingResult<T> = Result<T, ToolRoutingError>;

/// Context provided to tool-routing adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolRoutingContext {
    tenant: TenantId,
    backend: BackendId,
    conversation: Uuid,
    session: TurnSessionId,
}

impl ToolRoutingContext {
    /// Creates tool-routing context from turn execution metadata.
    #[must_use]
    pub const fn new(
        tenant_id: TenantId,
        backend_id: BackendId,
        conversation_id: Uuid,
        turn_session_id: TurnSessionId,
    ) -> Self {
        Self {
            tenant: tenant_id,
            backend: backend_id,
            conversation: conversation_id,
            session: turn_session_id,
        }
    }

    /// Returns tenant ID for tenant-scoped routing decisions.
    #[must_use]
    pub const fn tenant_id(self) -> TenantId {
        self.tenant
    }

    /// Returns backend ID for routing decisions.
    #[must_use]
    pub const fn backend_id(self) -> BackendId {
        self.backend
    }

    /// Returns conversation ID for routing decisions.
    #[must_use]
    pub const fn conversation_id(self) -> Uuid {
        self.conversation
    }

    /// Returns turn-session ID for routing decisions.
    #[must_use]
    pub const fn turn_session_id(self) -> TurnSessionId {
        self.session
    }
}

/// Port for routing tool calls through a single orchestration path.
#[async_trait]
pub trait ToolRouterPort: Send + Sync {
    /// Routes one tool call and returns the canonical tool result.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRoutingError`] when routing or execution fails.
    async fn route_tool_call(
        &self,
        call_id: &str,
        tool_call: &ToolCallRequest,
        context: ToolRoutingContext,
    ) -> ToolRoutingResult<ToolCallResult>;
}

/// Errors returned by tool-routing adapters.
#[derive(Debug, Error)]
pub enum ToolRoutingError {
    /// The requested tool name is not registered.
    #[error("tool not found: {0}")]
    ToolNotFound(String),

    /// Tool execution failed.
    #[error("tool execution failed: {0}")]
    ToolExecutionFailed(String),

    /// Infrastructure failure from the router adapter.
    #[error("tool router infrastructure error: {0}")]
    Infrastructure(Arc<dyn std::error::Error + Send + Sync>),
}

impl ToolRoutingError {
    /// Wraps an infrastructure-specific tool routing error.
    #[must_use]
    pub fn infrastructure(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Infrastructure(Arc::new(err))
    }
}
