//! Port contract for tool call policy enforcement.

use crate::tool_registry::domain::PolicyDecision;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use thiserror::Error;

/// Result type for policy evaluation operations.
pub type ToolPolicyResult<T> = Result<T, ToolPolicyError>;

/// Contract for evaluating tool call authorization policies.
///
/// Implementations check whether a given tool call should be permitted
/// before execution. The default wiring permits all calls; real
/// authorization will be layered in when workspace and user permission
/// systems exist.
#[async_trait]
pub trait ToolPolicyEnforcer: Send + Sync {
    /// Evaluates whether the given tool call should be permitted.
    ///
    /// # Errors
    ///
    /// Returns [`ToolPolicyError`] when the evaluation itself fails
    /// (distinct from a policy denial, which is a successful evaluation
    /// yielding [`PolicyDecision::Deny`]).
    async fn evaluate(
        &self,
        tool_name: &str,
        parameters: &Value,
    ) -> Result<PolicyDecision, ToolPolicyError>;
}

/// Errors returned when policy evaluation fails.
#[derive(Debug, Error)]
pub enum ToolPolicyError {
    /// The policy evaluation mechanism itself failed.
    #[error("policy evaluation failed: {0}")]
    EvaluationFailed(Arc<dyn std::error::Error + Send + Sync>),
}
