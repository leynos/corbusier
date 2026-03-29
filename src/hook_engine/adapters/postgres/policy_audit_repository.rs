//! `PostgreSQL` repository implementation for hook policy audit projections.

use super::policy_audit_models::{NewPolicyAuditEventRow, PolicyAuditEventRow};
use super::schema::hook_policy_audit_events;
use crate::context::{RequestContext, TenantId};
use crate::hook_engine::domain::{
    HookActionId, HookExecutionId, HookId, HookTriggerType, PolicyAuditDecision, PolicyAuditEvent,
    PolicyAuditEventId, PolicyViolation, TriggerContextId,
};
use crate::hook_engine::ports::{
    HookPolicyAuditError, HookPolicyAuditRepository, HookPolicyAuditResult,
};
use crate::message::adapters::postgres::tenant_tx::{FromTxError, TxError, with_tenant_tx};
use crate::message::domain::ConversationId;
use crate::task::domain::TaskId;
use async_trait::async_trait;
use diesel::pg::Pg;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

/// `PostgreSQL` connection pool type used by hook policy audit adapters.
pub type HookPolicyAuditPgPool = Pool<ConnectionManager<PgConnection>>;

/// `PostgreSQL`-backed hook policy audit repository.
#[derive(Debug, Clone)]
pub struct PostgresHookPolicyAuditRepository {
    pool: HookPolicyAuditPgPool,
}

impl PostgresHookPolicyAuditRepository {
    /// Creates a new repository from a `PostgreSQL` connection pool.
    #[must_use]
    pub const fn new(pool: HookPolicyAuditPgPool) -> Self {
        Self { pool }
    }

    async fn execute_query<F, T>(
        &self,
        tenant_id: TenantId,
        query_fn: F,
    ) -> HookPolicyAuditResult<T>
    where
        F: FnOnce(&mut PgConnection) -> HookPolicyAuditResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool
                .get()
                .map_err(HookPolicyAuditError::persistence_failed)?;
            with_tenant_tx(&mut connection, tenant_id.into_inner(), query_fn)
        })
        .await
        .map_err(HookPolicyAuditError::persistence_failed)?
    }

    async fn query_events_filtered<F>(
        &self,
        tenant_id: TenantId,
        extra_filter: F,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>
    where
        F: FnOnce(
                hook_policy_audit_events::BoxedQuery<'static, Pg>,
            ) -> hook_policy_audit_events::BoxedQuery<'static, Pg>
            + Send
            + 'static,
    {
        self.execute_query(tenant_id, move |connection| {
            let query = hook_policy_audit_events::table
                .filter(hook_policy_audit_events::tenant_id.eq(tenant_id.into_inner()))
                .into_boxed::<Pg>();
            let rows = extra_filter(query)
                .order_by((
                    hook_policy_audit_events::recorded_at.asc(),
                    hook_policy_audit_events::action_id.asc(),
                ))
                .select(PolicyAuditEventRow::as_select())
                .load::<PolicyAuditEventRow>(connection)
                .map_err(HookPolicyAuditError::persistence_failed)?;
            rows.into_iter().map(row_to_event).collect()
        })
        .await
    }
}

impl FromTxError<Self> for HookPolicyAuditError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(error) => error,
            TxError::Diesel(error) => Self::persistence_failed(error),
        }
    }
}

#[async_trait]
impl HookPolicyAuditRepository for PostgresHookPolicyAuditRepository {
    async fn store(
        &self,
        ctx: &RequestContext,
        event: &PolicyAuditEvent,
    ) -> HookPolicyAuditResult<()> {
        let tenant_id = ctx.tenant_id();
        let row = NewPolicyAuditEventRow {
            id: event.id().into_inner(),
            tenant_id: tenant_id.into_inner(),
            hook_execution_id: event.hook_execution_id().into_inner(),
            trigger_context_id: event.trigger_context_id().into_inner(),
            trigger_type: event.trigger_type().as_str().to_owned(),
            hook_id: event.hook_id().as_str().to_owned(),
            action_id: event.action_id().as_str().to_owned(),
            task_id: event.task_id().map(TaskId::into_inner),
            conversation_id: event.conversation_id().map(ConversationId::into_inner),
            decision: event.decision().as_str().to_owned(),
            violation: event
                .violation()
                .map(serde_json::to_value)
                .transpose()
                .map_err(HookPolicyAuditError::persistence_failed)?,
            payload: event.payload().clone(),
            recorded_at: event.recorded_at(),
        };

        self.execute_query(tenant_id, move |connection| {
            diesel::insert_into(hook_policy_audit_events::table)
                .values(&row)
                .on_conflict((
                    hook_policy_audit_events::tenant_id,
                    hook_policy_audit_events::hook_execution_id,
                    hook_policy_audit_events::action_id,
                ))
                .do_nothing()
                .execute(connection)
                .map_err(HookPolicyAuditError::persistence_failed)?;
            Ok(())
        })
        .await
    }

    async fn find_by_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        let tenant_id = ctx.tenant_id();
        let task_uuid = task_id.into_inner();
        self.query_events_filtered(tenant_id, move |query| {
            query.filter(hook_policy_audit_events::task_id.eq(task_uuid))
        })
        .await
    }

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        let tenant_id = ctx.tenant_id();
        let conversation_uuid = conversation_id.into_inner();
        self.query_events_filtered(tenant_id, move |query| {
            query.filter(hook_policy_audit_events::conversation_id.eq(conversation_uuid))
        })
        .await
    }

    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>> {
        let tenant_id = ctx.tenant_id();
        let trigger_context_uuid = trigger_context_id.into_inner();
        self.query_events_filtered(tenant_id, move |query| {
            query.filter(hook_policy_audit_events::trigger_context_id.eq(trigger_context_uuid))
        })
        .await
    }
}

fn row_to_event(row: PolicyAuditEventRow) -> HookPolicyAuditResult<PolicyAuditEvent> {
    let hook_id = HookId::new(row.hook_id)
        .map_err(|err| HookPolicyAuditError::invalid_persisted_data(err.to_string()))?;
    let action_id = HookActionId::new(row.action_id)
        .map_err(|err| HookPolicyAuditError::invalid_persisted_data(err.to_string()))?;
    let trigger_type = HookTriggerType::try_from(row.trigger_type.as_str())
        .map_err(|err| HookPolicyAuditError::invalid_persisted_data(err.to_string()))?;
    let decision = PolicyAuditDecision::try_from(row.decision.as_str())
        .map_err(|err| HookPolicyAuditError::invalid_persisted_data(err.to_string()))?;
    let violation = row
        .violation
        .map(serde_json::from_value::<PolicyViolation>)
        .transpose()
        .map_err(|err| HookPolicyAuditError::invalid_persisted_data(err.to_string()))?;

    Ok(PolicyAuditEvent {
        id: PolicyAuditEventId::from_uuid(row.id),
        hook_execution_id: HookExecutionId::from_uuid(row.hook_execution_id),
        trigger_context_id: TriggerContextId::from_uuid(row.trigger_context_id),
        trigger_type,
        hook_id,
        action_id,
        task_id: row.task_id.map(TaskId::from_uuid),
        conversation_id: row.conversation_id.map(ConversationId::from_uuid),
        decision,
        violation,
        payload: row.payload,
        recorded_at: row.recorded_at,
    })
}
