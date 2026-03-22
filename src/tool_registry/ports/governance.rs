//! Port contract for hook-backed tool execution governance.

use crate::context::RequestContext;
use crate::tool_registry::domain::{
    CatalogEntry, ToolCallRequest, ToolCallResult, ToolGovernanceDecision,
};
use async_trait::async_trait;
use thiserror::Error;

/// Result type for tool governance operations.
pub type ToolGovernanceResult<T> = Result<T, ToolGovernanceError>;

/// Bundles a completed tool call for post-execution observation.
pub struct CompletedToolCall<'a> {
    /// Original request submitted by the caller.
    pub request: &'a ToolCallRequest,
    /// Catalog entry that resolved the tool call.
    pub entry: &'a CatalogEntry,
    /// Tool execution result returned by the host.
    pub result: &'a ToolCallResult,
}

/// Alias retained for compatibility with earlier naming proposals.
pub type PostToolCallArgs<'a> = CompletedToolCall<'a>;

/// Alias retained for compatibility with earlier naming proposals.
pub type ToolObservationArgs<'a> = CompletedToolCall<'a>;

/// Alias retained for compatibility with earlier naming proposals.
pub type ToolCallObservation<'a> = CompletedToolCall<'a>;

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
    async fn observe_after_call(
        &self,
        ctx: &RequestContext,
        call: &CompletedToolCall<'_>,
    ) -> ToolGovernanceResult<()> {
        let _ = (ctx, call);
        Ok(())
    }
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
