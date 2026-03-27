//! In-memory tool router adapter for orchestrated agent turns.

use crate::agent_backend::{
    domain::{ToolCallRequest, ToolCallResult},
    ports::{
        ToolRouterPort, ToolRoutingContext, ToolRoutingError, ToolRoutingResult,
        tool_router::ToolRoutingInfrastructureError,
    },
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Default)]
struct InMemoryToolRouterState {
    responses: HashMap<String, Value>,
    failures: HashMap<String, String>,
    routed_call_ids: Vec<String>,
}

/// Thread-safe in-memory tool router.
#[derive(Debug, Clone, Default)]
pub struct InMemoryToolRouter {
    state: Arc<RwLock<InMemoryToolRouterState>>,
}

impl InMemoryToolRouter {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "RwLock poison errors are produced by value and converted immediately"
    )]
    fn map_lock_err<T>(err: std::sync::PoisonError<T>) -> ToolRoutingInfrastructureError {
        ToolRoutingInfrastructureError::AdapterUnavailable(err.to_string())
    }

    fn read_state(
        &self,
    ) -> ToolRoutingResult<std::sync::RwLockReadGuard<'_, InMemoryToolRouterState>> {
        Ok(self.state.read().map_err(Self::map_lock_err)?)
    }

    fn write_state(
        &self,
    ) -> ToolRoutingResult<std::sync::RwLockWriteGuard<'_, InMemoryToolRouterState>> {
        Ok(self.state.write().map_err(Self::map_lock_err)?)
    }

    /// Creates a new router with empty configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures a static output payload for a tool name.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRoutingError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn set_tool_response(
        &self,
        tool_name: impl Into<String>,
        output: Value,
    ) -> ToolRoutingResult<()> {
        let tool_name_key = tool_name.into().trim().to_owned();
        let mut state = self.write_state()?;
        state.failures.remove(&tool_name_key);
        state.responses.insert(tool_name_key, output);
        Ok(())
    }

    /// Configures a failure for a tool name.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRoutingError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn fail_tool(
        &self,
        tool_name: impl Into<String>,
        message: impl Into<String>,
    ) -> ToolRoutingResult<()> {
        let tool_name_key = tool_name.into().trim().to_owned();
        let mut state = self.write_state()?;
        state.responses.remove(&tool_name_key);
        state.failures.insert(tool_name_key, message.into());
        Ok(())
    }

    /// Returns call IDs in the order they were routed.
    ///
    /// # Errors
    ///
    /// Returns [`ToolRoutingError::Infrastructure`] when the in-memory state
    /// lock cannot be acquired.
    pub fn routed_call_ids(&self) -> ToolRoutingResult<Vec<String>> {
        let state = self.read_state()?;
        Ok(state.routed_call_ids.clone())
    }
}

#[async_trait]
impl ToolRouterPort for InMemoryToolRouter {
    async fn route_tool_call(
        &self,
        call_id: &str,
        tool_call: &ToolCallRequest,
        _context: ToolRoutingContext,
    ) -> ToolRoutingResult<ToolCallResult> {
        let mut state = self.write_state()?;
        state.routed_call_ids.push(call_id.to_owned());

        if let Some(message) = state.failures.get(tool_call.tool_name()) {
            return Err(ToolRoutingError::ToolExecutionFailed(message.to_owned()));
        }

        let output = state
            .responses
            .get(tool_call.tool_name())
            .cloned()
            .unwrap_or_else(|| tool_call.parameters().clone());

        Ok(ToolCallResult::new(call_id, tool_call.tool_name(), output))
    }
}
