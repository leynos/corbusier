//! Policy enforcement adapters for tool call authorisation.

use crate::tool_registry::{
    domain::PolicyDecision,
    ports::{ToolPolicyEnforcer, ToolPolicyError},
};
use async_trait::async_trait;
use serde_json::Value;

/// Policy adapter that unconditionally allows all tool calls.
///
/// This is the default policy adapter, providing an extensibility point
/// for future authorisation logic without blocking current functionality.
#[derive(Debug, Clone, Default)]
pub struct AllowAllPolicy;

#[async_trait]
impl ToolPolicyEnforcer for AllowAllPolicy {
    async fn evaluate(
        &self,
        _tool_name: &str,
        _parameters: &Value,
    ) -> Result<PolicyDecision, ToolPolicyError> {
        Ok(PolicyDecision::Allow)
    }
}

/// Policy adapter that unconditionally denies all tool calls.
///
/// Intended for testing policy enforcement paths.
#[derive(Debug, Clone)]
pub struct DenyAllPolicy {
    reason: String,
}

impl DenyAllPolicy {
    /// Creates a deny-all policy with the given reason.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ToolPolicyEnforcer for DenyAllPolicy {
    async fn evaluate(
        &self,
        _tool_name: &str,
        _parameters: &Value,
    ) -> Result<PolicyDecision, ToolPolicyError> {
        Ok(PolicyDecision::Deny {
            reason: self.reason.clone(),
        })
    }
}
