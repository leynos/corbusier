//! Policy audit projection types derived from hook execution results.

use super::{
    ActionResult, HookActionType, HookExecutionId, HookExecutionResult, HookId, HookTriggerContext,
    HookTriggerType, TriggerContextId,
};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;

/// Unique identifier for a policy audit event projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PolicyAuditEventId(Uuid);

impl PolicyAuditEventId {
    /// Creates a new random policy audit event identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an identifier from an existing UUID.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the wrapped UUID.
    #[must_use]
    pub const fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for PolicyAuditEventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PolicyAuditEventId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Decision captured by a policy audit event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAuditDecision {
    /// The policy check allowed execution to continue.
    Allow,
    /// The policy check denied execution.
    Deny,
}

impl PolicyAuditDecision {
    /// Returns the stable string representation for persistence.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }
}

impl TryFrom<&str> for PolicyAuditDecision {
    type Error = PolicyAuditProjectionError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "allow" => Ok(Self::Allow),
            "deny" => Ok(Self::Deny),
            other => Err(PolicyAuditProjectionError::invalid_decision(other)),
        }
    }
}

/// Stable policy violation details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyViolation {
    code: String,
    reason: String,
}

impl PolicyViolation {
    /// Creates a policy violation payload.
    ///
    /// # Errors
    ///
    /// Returns [`PolicyAuditProjectionError`] when the violation code or
    /// reason is empty after trimming.
    pub fn new(
        raw_code_input: impl Into<String>,
        raw_reason_input: impl Into<String>,
    ) -> Result<Self, PolicyAuditProjectionError> {
        let raw_code = raw_code_input.into();
        let raw_reason = raw_reason_input.into();
        let trimmed_code = raw_code.trim();
        let trimmed_reason = raw_reason.trim();
        if trimmed_code.is_empty() {
            return Err(PolicyAuditProjectionError::invalid_violation(
                "violation code must not be empty",
            ));
        }
        if trimmed_reason.is_empty() {
            return Err(PolicyAuditProjectionError::invalid_violation(
                "violation reason must not be empty",
            ));
        }
        Ok(Self {
            code: trimmed_code.to_owned(),
            reason: trimmed_reason.to_owned(),
        })
    }

    /// Returns the machine-readable violation code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the human-readable violation reason.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// Stored audit projection for a policy-check action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyAuditEvent {
    pub(crate) id: PolicyAuditEventId,
    pub(crate) hook_execution_id: HookExecutionId,
    pub(crate) trigger_context_id: TriggerContextId,
    pub(crate) trigger_type: HookTriggerType,
    pub(crate) hook_id: HookId,
    pub(crate) action_id: super::HookActionId,
    pub(crate) task_id: Option<TaskId>,
    pub(crate) conversation_id: Option<ConversationId>,
    pub(crate) decision: PolicyAuditDecision,
    pub(crate) violation: Option<PolicyViolation>,
    pub(crate) payload: serde_json::Value,
    pub(crate) recorded_at: DateTime<Utc>,
}

impl PolicyAuditEvent {
    /// Returns the event identifier.
    #[must_use]
    pub const fn id(&self) -> PolicyAuditEventId {
        self.id
    }

    /// Returns the hook execution identifier.
    #[must_use]
    pub const fn hook_execution_id(&self) -> HookExecutionId {
        self.hook_execution_id
    }

    /// Returns the trigger context identifier.
    #[must_use]
    pub const fn trigger_context_id(&self) -> TriggerContextId {
        self.trigger_context_id
    }

    /// Returns the trigger type.
    #[must_use]
    pub const fn trigger_type(&self) -> HookTriggerType {
        self.trigger_type
    }

    /// Returns the hook identifier.
    #[must_use]
    pub const fn hook_id(&self) -> &HookId {
        &self.hook_id
    }

    /// Returns the policy action identifier.
    #[must_use]
    pub const fn action_id(&self) -> &super::HookActionId {
        &self.action_id
    }

    /// Returns the correlated task identifier, if available.
    #[must_use]
    pub const fn task_id(&self) -> Option<TaskId> {
        self.task_id
    }

    /// Returns the correlated conversation identifier, if available.
    #[must_use]
    pub const fn conversation_id(&self) -> Option<ConversationId> {
        self.conversation_id
    }

    /// Returns the policy decision.
    #[must_use]
    pub const fn decision(&self) -> PolicyAuditDecision {
        self.decision
    }

    /// Returns the violation details, if any.
    #[must_use]
    pub const fn violation(&self) -> Option<&PolicyViolation> {
        self.violation.as_ref()
    }

    /// Returns the raw policy payload.
    #[must_use]
    pub const fn payload(&self) -> &serde_json::Value {
        &self.payload
    }

    /// Returns the timestamp when the event was recorded.
    #[must_use]
    pub const fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }
}

/// Error raised when a policy audit projection cannot be derived.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum PolicyAuditProjectionError {
    /// The projection contained an invalid decision value.
    #[error("invalid policy decision '{0}'")]
    InvalidDecision(String),
    /// The projection payload is structurally invalid.
    #[error("invalid policy output for action '{action_id}': {reason}")]
    InvalidOutput {
        /// Action identifier that produced the invalid output.
        action_id: String,
        /// Human-readable reason describing the invalid shape.
        reason: String,
    },
    /// The violation payload is malformed.
    #[error("invalid policy violation payload: {0}")]
    InvalidViolation(String),
}

impl PolicyAuditProjectionError {
    fn invalid_decision(value: impl Into<String>) -> Self {
        Self::InvalidDecision(value.into())
    }

    fn invalid_output(action_id: &super::HookActionId, reason: impl Into<String>) -> Self {
        Self::InvalidOutput {
            action_id: action_id.as_str().to_owned(),
            reason: reason.into(),
        }
    }

    fn invalid_violation(reason: impl Into<String>) -> Self {
        Self::InvalidViolation(reason.into())
    }
}

/// Projects policy audit events from a completed hook execution.
///
/// # Errors
///
/// Returns [`PolicyAuditProjectionError`] when a `PolicyCheck` action emits an
/// invalid output payload.
pub fn project_policy_audit_events(
    result: &HookExecutionResult,
    context: &HookTriggerContext,
) -> Result<Vec<PolicyAuditEvent>, PolicyAuditProjectionError> {
    result
        .action_results()
        .iter()
        .filter(|action| matches!(action.action_type(), HookActionType::PolicyCheck))
        .map(|action| project_policy_audit_event(result, context, action))
        .collect()
}

fn project_policy_audit_event(
    result: &HookExecutionResult,
    context: &HookTriggerContext,
    action: &ActionResult,
) -> Result<PolicyAuditEvent, PolicyAuditProjectionError> {
    let object = action.output().as_object().ok_or_else(|| {
        PolicyAuditProjectionError::invalid_output(action.action_id(), "expected an object payload")
    })?;
    let decision_raw = object
        .get("decision")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            PolicyAuditProjectionError::invalid_output(
                action.action_id(),
                "missing string field 'decision'",
            )
        })?;
    let decision = PolicyAuditDecision::try_from(decision_raw)?;
    let violation = parse_violation(object, decision, action.action_id())?;
    Ok(PolicyAuditEvent {
        id: PolicyAuditEventId::new(),
        hook_execution_id: result.execution_id(),
        trigger_context_id: result.trigger_context_id(),
        trigger_type: result.trigger_type(),
        hook_id: result.hook_id().clone(),
        action_id: action.action_id().clone(),
        task_id: context.execution_scope().task_id(),
        conversation_id: context.execution_scope().conversation_id(),
        decision,
        violation,
        payload: action.output().clone(),
        recorded_at: result.executed_at(),
    })
}

fn parse_violation(
    object: &serde_json::Map<String, serde_json::Value>,
    decision: PolicyAuditDecision,
    action_id: &super::HookActionId,
) -> Result<Option<PolicyViolation>, PolicyAuditProjectionError> {
    if let Some(violation_value) = object.get("violation") {
        let violation = violation_value.as_object().ok_or_else(|| {
            PolicyAuditProjectionError::invalid_output(
                action_id,
                "field 'violation' must be an object",
            )
        })?;
        let code = violation
            .get("code")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                PolicyAuditProjectionError::invalid_output(
                    action_id,
                    "field 'violation.code' must be a string",
                )
            })?;
        let reason = violation
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                PolicyAuditProjectionError::invalid_output(
                    action_id,
                    "field 'violation.reason' must be a string",
                )
            })?;
        return PolicyViolation::new(code, reason).map(Some);
    }

    if decision == PolicyAuditDecision::Deny
        && let Some(reason) = object.get("reason").and_then(serde_json::Value::as_str)
    {
        return PolicyViolation::new("policy_denied", reason).map(Some);
    }

    Ok(None)
}
