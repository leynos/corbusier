//! In-memory repository for hook policy audit projections.

use crate::context::{RequestContext, TenantId};
use crate::hook_engine::domain::{PolicyAuditEvent, TriggerContextId};
use crate::hook_engine::ports::{HookPolicyAuditRepository, HookPolicyAuditResult};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe in-memory hook policy audit repository.
#[derive(Debug, Clone, Default)]
pub struct InMemoryHookPolicyAuditRepository {
    events: Arc<RwLock<HashMap<TenantId, Vec<PolicyAuditEvent>>>>,
}

#[derive(Clone)]
enum QueryKey {
    Task(TaskId),
    Conversation(ConversationId),
    Trigger(TriggerContextId),
}

impl InMemoryHookPolicyAuditRepository {
    /// Creates an empty in-memory policy audit repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn sort_events(events: &mut [PolicyAuditEvent]) {
        events.sort_by(|left, right| {
            left.recorded_at()
                .cmp(&right.recorded_at())
                .then_with(|| left.action_id().cmp(right.action_id()))
        });
    }

    async fn filter_tenant_events<F>(
        &self,
        ctx: &RequestContext,
        predicate: F,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>
    where
        F: Fn(&PolicyAuditEvent) -> bool,
    {
        let events = self.events.read().await;
        let tenant_events = events.get(&ctx.tenant_id()).map_or(&[][..], Vec::as_slice);
        let mut filtered = tenant_events
            .iter()
            .filter(|event| predicate(event))
            .cloned()
            .collect::<Vec<_>>();
        Self::sort_events(&mut filtered);
        Ok(filtered)
    }

    async fn query_by_key(
        &self,
        ctx: &RequestContext,
        key: QueryKey,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        self.filter_tenant_events(ctx, {
            let query_key = key.clone();
            move |event| match &query_key {
                QueryKey::Task(task_id) => event.task_id() == Some(*task_id),
                QueryKey::Conversation(conversation_id) => {
                    event.conversation_id() == Some(*conversation_id)
                }
                QueryKey::Trigger(trigger_context_id) => {
                    event.trigger_context_id() == *trigger_context_id
                }
            }
        })
        .await
    }
}

#[async_trait]
impl HookPolicyAuditRepository for InMemoryHookPolicyAuditRepository {
    async fn store(
        &self,
        ctx: &RequestContext,
        event: &PolicyAuditEvent,
    ) -> HookPolicyAuditResult<()> {
        let mut events = self.events.write().await;
        let tenant_events = events.entry(ctx.tenant_id()).or_default();
        if tenant_events.iter().any(|stored| {
            stored.hook_execution_id() == event.hook_execution_id()
                && stored.action_id() == event.action_id()
        }) {
            return Ok(());
        }
        tenant_events.push(event.clone());
        Ok(())
    }

    async fn find_by_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        self.query_by_key(ctx, QueryKey::Task(task_id)).await
    }

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        self.query_by_key(ctx, QueryKey::Conversation(conversation_id))
            .await
    }

    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        self.query_by_key(ctx, QueryKey::Trigger(trigger_context_id))
            .await
    }
}
