//! Governance adapters for tool execution authorization and audit observation.

use crate::context::RequestContext;
use crate::hook_engine::domain::{HookExecutionScope, HookTriggerContext, HookTriggerType};
use crate::hook_engine::ports::{HookEngine, HookPolicyAuditRepository};
use crate::tool_registry::domain::{
    CatalogEntry, ToolCallRequest, ToolCallResult, ToolGovernanceDecision,
};
use crate::tool_registry::ports::{
    CompletedToolCall, ToolExecutionGovernance, ToolGovernanceError,
};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Governance adapter that unconditionally allows all tool calls.
///
/// This is the default governance adapter, providing an extensibility point
/// for hook-backed enforcement without blocking current functionality.
#[derive(Debug, Clone, Default)]
pub struct AllowAllPolicy;

#[async_trait]
impl ToolExecutionGovernance for AllowAllPolicy {
    async fn enforce_before_call(
        &self,
        _ctx: &RequestContext,
        _request: &ToolCallRequest,
        _entry: &CatalogEntry,
    ) -> Result<ToolGovernanceDecision, ToolGovernanceError> {
        Ok(ToolGovernanceDecision::Allow)
    }
}

/// Governance adapter that unconditionally denies all tool calls.
///
/// Intended for testing pre-execution denial paths.
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
impl ToolExecutionGovernance for DenyAllPolicy {
    async fn enforce_before_call(
        &self,
        _ctx: &RequestContext,
        _request: &ToolCallRequest,
        _entry: &CatalogEntry,
    ) -> Result<ToolGovernanceDecision, ToolGovernanceError> {
        Ok(ToolGovernanceDecision::Deny {
            reason: self.reason.clone(),
        })
    }
}

/// Governance adapter that simulates a governance evaluation failure.
///
/// Intended for testing error propagation when the policy engine
/// itself fails (distinct from a governance denial decision).
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
impl ToolExecutionGovernance for FailingPolicy {
    async fn enforce_before_call(
        &self,
        _ctx: &RequestContext,
        _request: &ToolCallRequest,
        _entry: &CatalogEntry,
    ) -> Result<ToolGovernanceDecision, ToolGovernanceError> {
        Err(ToolGovernanceError::EvaluationFailed {
            message: self.message.clone(),
        })
    }

    async fn observe_after_call(
        &self,
        _ctx: &RequestContext,
        _call: &CompletedToolCall<'_>,
    ) -> Result<(), ToolGovernanceError> {
        Err(ToolGovernanceError::EvaluationFailed {
            message: self.message.clone(),
        })
    }
}

/// Governance adapter that delegates enforcement and observation to the hook
/// engine and policy audit repository.
#[derive(Debug, Clone)]
pub struct HookBackedToolExecutionGovernance<E, R>
where
    E: HookEngine,
    R: HookPolicyAuditRepository,
{
    hook_engine: Arc<E>,
    policy_audit_repository: Arc<R>,
}

impl<E, R> HookBackedToolExecutionGovernance<E, R>
where
    E: HookEngine,
    R: HookPolicyAuditRepository,
{
    /// Creates a new hook-backed governance adapter.
    #[must_use]
    pub const fn new(hook_engine: Arc<E>, policy_audit_repository: Arc<R>) -> Self {
        Self {
            hook_engine,
            policy_audit_repository,
        }
    }

    fn build_scope_metadata(
        request: &ToolCallRequest,
        entry: &CatalogEntry,
        result: Option<&ToolCallResult>,
    ) -> serde_json::Value {
        json!({
            "call_id": request.call_id().to_string(),
            "tool_name": request.tool_name(),
            "server_id": entry.server_id().to_string(),
            "parameters": request.parameters(),
            "workflow_metadata": request.execution_scope().metadata(),
            "result": result.map(ToolCallResult::outcome),
        })
    }

    fn build_trigger_context(
        trigger_type: HookTriggerType,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
        result: Option<&ToolCallResult>,
    ) -> HookTriggerContext {
        let base_scope = HookExecutionScope::default()
            .with_metadata(Self::build_scope_metadata(request, entry, result));
        let task_scope = if let Some(task_id) = request.execution_scope().task_id() {
            base_scope.with_task_id(task_id)
        } else {
            base_scope
        };
        let execution_scope =
            if let Some(conversation_id) = request.execution_scope().conversation_id() {
                task_scope.with_conversation_id(conversation_id)
            } else {
                task_scope
            };
        let occurred_at =
            result.map_or_else(|| request.initiated_at(), ToolCallResult::completed_at);
        HookTriggerContext::new_with_timestamp(trigger_type, execution_scope, occurred_at)
    }
}

#[async_trait]
impl<E, R> ToolExecutionGovernance for HookBackedToolExecutionGovernance<E, R>
where
    E: HookEngine,
    R: HookPolicyAuditRepository,
{
    async fn enforce_before_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
    ) -> Result<ToolGovernanceDecision, ToolGovernanceError> {
        let trigger_context =
            Self::build_trigger_context(HookTriggerType::PreToolUse, request, entry, None);
        let trigger_context_id = trigger_context.id();
        self.hook_engine
            .execute(ctx, trigger_context)
            .await
            .map_err(|err| ToolGovernanceError::EvaluationFailed {
                message: err.to_string(),
            })?;
        let events = self
            .policy_audit_repository
            .find_by_trigger_context(ctx, trigger_context_id)
            .await
            .map_err(|err| ToolGovernanceError::EvaluationFailed {
                message: err.to_string(),
            })?;

        Ok(
            denial_reason(events).map_or(ToolGovernanceDecision::Allow, |reason| {
                ToolGovernanceDecision::Deny { reason }
            }),
        )
    }

    async fn observe_after_call(
        &self,
        ctx: &RequestContext,
        call: &CompletedToolCall<'_>,
    ) -> Result<(), ToolGovernanceError> {
        let trigger_context = Self::build_trigger_context(
            HookTriggerType::PostToolUse,
            call.request,
            call.entry,
            Some(call.result),
        );
        self.hook_engine
            .execute(ctx, trigger_context)
            .await
            .map(|_| ())
            .map_err(|err| ToolGovernanceError::EvaluationFailed {
                message: err.to_string(),
            })
    }
}

fn denial_reason(events: Vec<crate::hook_engine::domain::PolicyAuditEvent>) -> Option<String> {
    events.into_iter().find_map(|event| {
        if matches!(
            event.decision(),
            crate::hook_engine::domain::PolicyAuditDecision::Deny
        ) {
            Some(event.violation().map_or_else(
                || "policy denied tool call".to_owned(),
                |violation| violation.reason().to_owned(),
            ))
        } else {
            None
        }
    })
}
