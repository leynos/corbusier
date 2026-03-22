//! In-memory tool router adapter for orchestrated agent turns.

use crate::agent_backend::{
    domain::{ToolCallRequest, ToolCallResult},
    ports::{ToolRouterPort, ToolRoutingContext, ToolRoutingError, ToolRoutingResult},
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
        let tool_name_key = tool_name.into();
        let mut state = self.state.write().map_err(|err| {
            ToolRoutingError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
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
        let tool_name_key = tool_name.into();
        let mut state = self.state.write().map_err(|err| {
            ToolRoutingError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
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
        let state = self.state.read().map_err(|err| {
            ToolRoutingError::infrastructure(std::io::Error::other(err.to_string()))
        })?;
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
        let mut state = self.state.write().map_err(|err| {
            ToolRoutingError::infrastructure(std::io::Error::other(err.to_string()))
        })?;

        if let Some(message) = state.failures.get(tool_call.tool_name()) {
            return Err(ToolRoutingError::ToolExecutionFailed(message.clone()));
        }

        let output = state
            .responses
            .get(tool_call.tool_name())
            .cloned()
            .unwrap_or_else(|| tool_call.parameters().clone());

        state.routed_call_ids.push(call_id.to_owned());

        Ok(ToolCallResult::new(call_id, tool_call.tool_name(), output))
    }
}
