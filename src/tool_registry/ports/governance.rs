//! Port contract for hook-backed tool execution governance.

use crate::context::RequestContext;
use crate::tool_registry::domain::{
    CatalogEntry, ToolCallRequest, ToolCallResult, ToolGovernanceDecision,
};
use async_trait::async_trait;
use thiserror::Error;

/// Result type for tool governance operations.
pub type ToolGovernanceResult<T> = Result<T, ToolGovernanceError>;

/// Contract for enforcing and observing tool execution governance.
#[async_trait]
pub trait ToolExecutionGovernance: Send + Sync {
    /// Evaluates whether a tool call should proceed before host execution.
    async fn enforce_before_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
    ) -> ToolGovernanceResult<ToolGovernanceDecision>;

    /// Observes a completed tool call after host execution and audit capture.
    #[expect(
        clippy::too_many_arguments,
        reason = "Post-call governance needs request, catalog entry, and result context."
    )]
    async fn observe_after_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
        result: &ToolCallResult,
    ) -> ToolGovernanceResult<()>;
}

/// Errors returned by tool governance adapters.
#[derive(Debug, Error)]
pub enum ToolGovernanceError {
    /// Governance evaluation failed.
    #[error("tool governance failed: {message}")]
    EvaluationFailed {
        /// Human-readable failure reason.
        message: String,
    },
}
