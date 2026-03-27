//! Governance adapters for tool execution authorization and audit observation.

use super::policy_metadata::build_scope_metadata;
use crate::context::RequestContext;
use crate::hook_engine::domain::{HookExecutionScope, HookTriggerContext, HookTriggerType};
use crate::hook_engine::ports::{HookEngine, HookPolicyAuditRepository};
use crate::tool_registry::domain::{
    CatalogEntry, ToolCallRequest, ToolCallResult, ToolExecutionScope, ToolGovernanceDecision,
};
use crate::tool_registry::ports::{
    CompletedToolCall, ToolExecutionGovernance, ToolGovernanceError,
};
use async_trait::async_trait;
use std::sync::Arc;

/// The fixed outcome returned by a [`StubGovernance`] adapter.
#[derive(Debug, Clone)]
pub enum StubOutcome {
    Allow,
    Deny { reason: String },
    Fail { message: String },
}

impl StubOutcome {
    fn before_call(&self) -> Result<ToolGovernanceDecision, ToolGovernanceError> {
        match self {
            Self::Allow => Ok(ToolGovernanceDecision::Allow),
            Self::Deny { reason } => Ok(ToolGovernanceDecision::Deny {
                reason: reason.clone(),
            }),
            Self::Fail { message } => Err(ToolGovernanceError::EvaluationFailed {
                message: message.clone(),
            }),
        }
    }

    fn after_call(&self) -> Result<(), ToolGovernanceError> {
        match self {
            Self::Fail { message } => Err(ToolGovernanceError::EvaluationFailed {
                message: message.clone(),
            }),
            Self::Allow | Self::Deny { .. } => Ok(()),
        }
    }
}

/// Stub governance adapter that returns a fixed outcome regardless of request inputs.
#[derive(Debug, Clone)]
pub struct StubGovernance {
    outcome: StubOutcome,
}

impl StubGovernance {
    /// Creates a stub that always allows tool execution.
    #[must_use]
    pub const fn allowing() -> Self {
        Self {
            outcome: StubOutcome::Allow,
        }
    }

    /// Creates a stub that always denies tool execution with the given reason.
    #[must_use]
    pub fn denying(reason: impl Into<String>) -> Self {
        Self {
            outcome: StubOutcome::Deny {
                reason: reason.into(),
            },
        }
    }

    /// Creates a stub that always fails governance evaluation with the given message.
    #[must_use]
    pub fn failing(message: impl Into<String>) -> Self {
        Self {
            outcome: StubOutcome::Fail {
                message: message.into(),
            },
        }
    }
}

impl Default for StubGovernance {
    fn default() -> Self {
        Self::allowing()
    }
}

#[async_trait]
impl ToolExecutionGovernance for StubGovernance {
    async fn enforce_before_call(
        &self,
        _ctx: &RequestContext,
        _request: &ToolCallRequest,
        _entry: &CatalogEntry,
    ) -> Result<ToolGovernanceDecision, ToolGovernanceError> {
        self.outcome.before_call()
    }

    async fn observe_after_call(
        &self,
        _ctx: &RequestContext,
        _call: &CompletedToolCall<'_>,
    ) -> Result<(), ToolGovernanceError> {
        self.outcome.after_call()
    }
}

macro_rules! legacy_stub_policy {
    (
        $(#[$doc:meta])*
        $name:ident,
        $ctor_doc:literal,
        $ctor:expr,
        $default:expr
    ) => {
        $(#[$doc])*
        #[derive(Debug, Clone)]
        pub struct $name(StubGovernance);

        impl $name {
            #[doc = $ctor_doc]
            #[must_use]
            pub fn new(message: impl Into<String>) -> Self { Self($ctor(message)) }
        }

        impl Default for $name {
            fn default() -> Self { Self::new($default) }
        }

        impl From<StubGovernance> for $name {
            fn from(governance: StubGovernance) -> Self { Self(governance) }
        }

        impl From<$name> for StubGovernance {
            fn from(policy: $name) -> Self { policy.0 }
        }

        #[async_trait]
        impl ToolExecutionGovernance for $name {
            async fn enforce_before_call(
                &self,
                ctx: &RequestContext,
                request: &ToolCallRequest,
                entry: &CatalogEntry,
            ) -> Result<ToolGovernanceDecision, ToolGovernanceError> {
                self.0.enforce_before_call(ctx, request, entry).await
            }

            async fn observe_after_call(
                &self,
                ctx: &RequestContext,
                call: &CompletedToolCall<'_>,
            ) -> Result<(), ToolGovernanceError> {
                self.0.observe_after_call(ctx, call).await
            }
        }
    };
}

/// Backwards-compatible wrapper for the always-allowing stub adapter.
#[derive(Debug, Clone)]
pub struct AllowAllPolicy(StubGovernance);

impl AllowAllPolicy {
    /// Creates a governance adapter that always allows tool execution.
    #[must_use]
    pub const fn new() -> Self {
        Self(StubGovernance::allowing())
    }
}

impl Default for AllowAllPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl From<StubGovernance> for AllowAllPolicy {
    fn from(governance: StubGovernance) -> Self {
        Self(governance)
    }
}

impl From<AllowAllPolicy> for StubGovernance {
    fn from(policy: AllowAllPolicy) -> Self {
        policy.0
    }
}

#[async_trait]
impl ToolExecutionGovernance for AllowAllPolicy {
    async fn enforce_before_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
    ) -> Result<ToolGovernanceDecision, ToolGovernanceError> {
        self.0.enforce_before_call(ctx, request, entry).await
    }

    async fn observe_after_call(
        &self,
        ctx: &RequestContext,
        call: &CompletedToolCall<'_>,
    ) -> Result<(), ToolGovernanceError> {
        self.0.observe_after_call(ctx, call).await
    }
}

legacy_stub_policy!(
    /// Backwards-compatible wrapper for the always-denying stub adapter.
    DenyAllPolicy,
    "Creates a governance adapter that always denies tool execution.",
    StubGovernance::denying,
    "tool execution denied"
);

legacy_stub_policy!(
    /// Backwards-compatible wrapper for the always-failing stub adapter.
    FailingPolicy,
    "Creates a governance adapter that always fails evaluation.",
    StubGovernance::failing,
    "tool governance failed"
);

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
        build_scope_metadata(request, entry, result)
    }

    fn project_execution_scope(
        scope: &ToolExecutionScope,
        metadata: serde_json::Value,
    ) -> HookExecutionScope {
        let mut hook_scope = HookExecutionScope::default().with_metadata(metadata);
        if let Some(task_id) = scope.task_id() {
            hook_scope = hook_scope.with_task_id(task_id);
        }
        if let Some(conversation_id) = scope.conversation_id() {
            hook_scope = hook_scope.with_conversation_id(conversation_id);
        }
        hook_scope
    }

    fn build_trigger_context(
        trigger_type: HookTriggerType,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
        result: Option<&ToolCallResult>,
    ) -> HookTriggerContext {
        let execution_scope = Self::project_execution_scope(
            request.execution_scope(),
            Self::build_scope_metadata(request, entry, result),
        );
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
    use crate::hook_engine::domain::PolicyAuditDecision::Deny;

    for event in events {
        if matches!(event.decision(), Deny) {
            let message = event.violation().map_or_else(
                || "policy denied tool call".to_owned(),
                |violation| violation.reason().to_owned(),
            );
            return Some(message);
        }
    }
    None
}
