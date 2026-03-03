//! Domain types for hook engine definitions and execution results.

pub mod action;
pub mod definition;
pub mod error;
pub mod execution;
pub mod ids;
pub mod trigger;

pub use action::{HookAction, HookActionType};
pub use definition::{HookDefinition, HookPredicate, HookPriority};
pub use error::HookDomainError;
pub use execution::{
    ActionResult, ActionStatus, HookExecutionResult, HookExecutionStatus, HookLogEntry,
    HookLogLevel,
};
pub use ids::{HookActionId, HookExecutionId, HookId, TriggerContextId};
pub use trigger::{HookTriggerContext, HookTriggerType, ParseHookTriggerTypeError};
