//! Port contract for persisting and querying hook policy audit projections.

use crate::context::RequestContext;
use crate::hook_engine::domain::{PolicyAuditEvent, TriggerContextId};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use async_trait::async_trait;
use thiserror::Error;

/// Result type for hook policy audit operations.
pub type HookPolicyAuditResult<T> = Result<T, HookPolicyAuditError>;

/// Hook-owned persistence and query contract for policy audit events.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait HookPolicyAuditRepository: Send + Sync {
    /// Stores a policy audit event projection.
    async fn store(
        &self,
        ctx: &RequestContext,
        event: &PolicyAuditEvent,
    ) -> HookPolicyAuditResult<()>;

    /// Returns policy audit events associated with a task.
    async fn find_by_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;

    /// Returns policy audit events associated with a conversation.
    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;

    /// Returns policy audit events associated with a trigger occurrence.
    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;
}

/// Errors returned by hook policy audit implementations.
#[derive(Debug, Clone, Error)]
pub enum HookPolicyAuditError {
    /// Persistence-layer failure.
    #[error("policy audit persistence operation failed: {reason}")]
    PersistenceFailed {
        /// Human-readable reason from the failing dependency.
        reason: String,
    },
    /// Persisted data failed validation.
    #[error("invalid persisted hook policy audit data: {0}")]
    InvalidPersistedData(String),
}

impl HookPolicyAuditError {
    /// Creates a persistence failure from an infrastructure error.
    pub fn persistence_failed(err: impl std::error::Error) -> Self {
        Self::PersistenceFailed {
            reason: err.to_string(),
        }
    }

    /// Creates an invalid persisted data error.
    pub fn invalid_persisted_data(err: impl Into<String>) -> Self {
        Self::InvalidPersistedData(err.into())
    }
}
