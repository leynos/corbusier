//! Policy enforcement adapters for tool call authorization.

use crate::tool_registry::{
    domain::PolicyDecision,
    ports::{ToolPolicyEnforcer, ToolPolicyError},
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// Policy adapter that unconditionally allows all tool calls.
///
/// This is the default policy adapter, providing an extensibility point
/// for future authorization logic without blocking current functionality.
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

/// Policy adapter that simulates a policy evaluation failure.
///
/// Intended for testing error propagation when the policy engine
/// itself fails (distinct from a policy denial decision).
#[derive(Debug, Clone)]
pub struct FailingPolicy {
    message: String,
}

impl FailingPolicy {
    /// Creates a failing policy with the given error message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[async_trait]
impl ToolPolicyEnforcer for FailingPolicy {
    async fn evaluate(
        &self,
        _tool_name: &str,
        _parameters: &Value,
    ) -> Result<PolicyDecision, ToolPolicyError> {
        Err(ToolPolicyError::EvaluationFailed(Arc::from(
            std::io::Error::other(&*self.message),
        )))
    }
}
