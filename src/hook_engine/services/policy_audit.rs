//! Query service for hook policy audit projections.

use crate::context::RequestContext;
use crate::hook_engine::domain::{PolicyAuditEvent, TriggerContextId};
use crate::hook_engine::ports::{HookPolicyAuditRepository, HookPolicyAuditResult};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use std::sync::Arc;

/// Read-oriented service wrapping the policy audit repository.
#[derive(Clone)]
pub struct HookPolicyAuditQueryService<R>
where
    R: HookPolicyAuditRepository,
{
    repository: Arc<R>,
}

impl<R> HookPolicyAuditQueryService<R>
where
    R: HookPolicyAuditRepository,
{
    /// Creates a new query service.
    #[must_use]
    pub const fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Returns policy audit events associated with a task.
    ///
    /// # Errors
    ///
    /// Returns a policy-audit repository error when the lookup fails.
    pub async fn find_by_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        self.repository.find_by_task(ctx, task_id).await
    }

    /// Returns policy audit events associated with a conversation.
    ///
    /// # Errors
    ///
    /// Returns a policy-audit repository error when the lookup fails.
    pub async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        self.repository
            .find_by_conversation(ctx, conversation_id)
            .await
    }

    /// Returns policy audit events associated with a trigger occurrence.
    ///
    /// # Errors
    ///
    /// Returns a policy-audit repository error when the lookup fails.
    pub async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        self.repository
            .find_by_trigger_context(ctx, trigger_context_id)
            .await
    }
}
