//! Domain types for hook engine definitions and execution results.

pub mod action;
pub mod definition;
pub mod error;
pub mod execution;
pub mod ids;
pub mod policy_audit;
pub mod trigger;

pub use action::{HookAction, HookActionType};
pub use definition::{HookDefinition, HookPredicate, HookPriority};
pub use error::HookDomainError;
pub use execution::{
    ActionResult, ActionResultDetails, ActionStatus, HookExecutionInput, HookExecutionPersisted,
    HookExecutionResult, HookExecutionStatus, HookLogEntry, HookLogLevel,
};
pub use ids::{HookActionId, HookExecutionId, HookId, TriggerContextId};
pub use policy_audit::{
    PolicyAuditDecision, PolicyAuditEvent, PolicyAuditEventId, PolicyAuditEventInput,
    PolicyAuditProjectionError, PolicyViolation, project_policy_audit_events,
};
pub use trigger::{
    HookExecutionScope, HookTriggerContext, HookTriggerType, ParseHookTriggerTypeError,
};
