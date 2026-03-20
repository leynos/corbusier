//! `PostgreSQL` repository implementation for hook execution logs.

use super::models::{HookExecutionRow, NewHookExecutionRow};
use super::schema::hook_executions;
use crate::context::{RequestContext, TenantId};
use crate::hook_engine::domain::{
    ActionResult, HookExecutionId, HookExecutionPersisted, HookExecutionResult,
    HookExecutionStatus, HookId, HookTriggerType, TriggerContextId,
};
use crate::hook_engine::ports::{
    HookExecutionLogError, HookExecutionLogRepository, HookExecutionLogResult,
    PendingExecutionRecord,
};
use crate::message::adapters::postgres::tenant_tx::{FromTxError, TxError, with_tenant_tx};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::upsert::excluded;

/// `PostgreSQL` connection pool type used by hook execution log adapters.
pub type HookExecutionPgPool = Pool<ConnectionManager<PgConnection>>;

/// `PostgreSQL`-backed hook execution log repository.
#[derive(Debug, Clone)]
pub struct PostgresHookExecutionLogRepository {
    pool: HookExecutionPgPool,
}

impl PostgresHookExecutionLogRepository {
    /// Creates a new repository from a `PostgreSQL` connection pool.
    ///
    /// Example: `PostgresHookExecutionLogRepository::new(pool)` wraps the pool.
    #[must_use]
    pub const fn new(pool: HookExecutionPgPool) -> Self {
        Self { pool }
    }

    async fn execute_query<F, T>(
        &self,
        tenant_id: TenantId,
        query_fn: F,
    ) -> HookExecutionLogResult<T>
    where
        F: FnOnce(&mut PgConnection) -> HookExecutionLogResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool
                .get()
                .map_err(HookExecutionLogError::persistence_failed)?;
            with_tenant_tx(&mut connection, tenant_id.into_inner(), query_fn)
        })
        .await
        .map_err(HookExecutionLogError::persistence_failed)?
    }
}

impl FromTxError<Self> for HookExecutionLogError {
    fn from_tx_error(err: TxError<Self>) -> Self {
        match err {
            TxError::Domain(error) => error,
            TxError::Diesel(error) => Self::persistence_failed(error),
        }
    }
}

#[async_trait]
impl HookExecutionLogRepository for PostgresHookExecutionLogRepository {
    async fn store_pending(
        &self,
        ctx: &RequestContext,
        record: PendingExecutionRecord,
    ) -> HookExecutionLogResult<()> {
        let tenant_id = ctx.tenant_id();
        let new_row = NewHookExecutionRow {
            id: record.execution_id.into_inner(),
            tenant_id: tenant_id.into_inner(),
            trigger_context_id: record.trigger_context_id.into_inner(),
            hook_id: record.hook_id.as_str().to_owned(),
            trigger_type: record.trigger_type.as_str().to_owned(),
            predicate_data: serde_json::Value::Object(serde_json::Map::new()),
            action_results: serde_json::Value::Array(Vec::new()),
            status: HookExecutionStatus::Pending.as_str().to_owned(),
            executed_at: record.executed_at,
        };

        self.execute_query(tenant_id, move |connection| {
            diesel::insert_into(hook_executions::table)
                .values(&new_row)
                .on_conflict((
                    hook_executions::tenant_id,
                    hook_executions::trigger_context_id,
                    hook_executions::hook_id,
                ))
                .do_update()
                .set((
                    hook_executions::id.eq(excluded(hook_executions::id)),
                    hook_executions::trigger_type.eq(excluded(hook_executions::trigger_type)),
                    hook_executions::predicate_data.eq(excluded(hook_executions::predicate_data)),
                    hook_executions::action_results.eq(excluded(hook_executions::action_results)),
                    hook_executions::status.eq(excluded(hook_executions::status)),
                    hook_executions::executed_at.eq(excluded(hook_executions::executed_at)),
                ))
                .execute(connection)
                .map_err(HookExecutionLogError::persistence_failed)?;
            Ok(())
        })
        .await
    }

    async fn update_result(
        &self,
        ctx: &RequestContext,
        result: &HookExecutionResult,
    ) -> HookExecutionLogResult<()> {
        let tenant_id = ctx.tenant_id();
        let owned_result = result.clone();
        let execution_id = owned_result.execution_id().into_inner();
        let action_results = serde_json::to_value(owned_result.action_results())
            .map_err(HookExecutionLogError::persistence_failed)?;
        let hook_id = owned_result.hook_id().as_str().to_owned();
        let trigger_type = owned_result.trigger_type().as_str().to_owned();
        let predicate_data = owned_result.predicate_data().clone();
        let status = owned_result.status().as_str().to_owned();
        let executed_at = owned_result.executed_at();
        let trigger_context_id = owned_result.trigger_context_id().into_inner();
        let execution_id_text = owned_result.execution_id().to_string();

        self.execute_query(tenant_id, move |connection| {
            let updated_rows = diesel::update(
                hook_executions::table
                    .filter(hook_executions::tenant_id.eq(tenant_id.into_inner()))
                    .filter(hook_executions::id.eq(execution_id)),
            )
            .set((
                hook_executions::trigger_context_id.eq(trigger_context_id),
                hook_executions::hook_id.eq(hook_id),
                hook_executions::trigger_type.eq(trigger_type),
                hook_executions::predicate_data.eq(predicate_data),
                hook_executions::action_results.eq(action_results),
                hook_executions::status.eq(status),
                hook_executions::executed_at.eq(executed_at),
            ))
            .execute(connection)
            .map_err(HookExecutionLogError::persistence_failed)?;

            if updated_rows == 0 {
                return Err(HookExecutionLogError::invalid_persisted_data(format!(
                    "missing pending hook execution for {}",
                    execution_id_text
                )));
            }

            Ok(())
        })
        .await
    }

    async fn store(
        &self,
        ctx: &RequestContext,
        result: &HookExecutionResult,
    ) -> HookExecutionLogResult<()> {
        let tenant_id = ctx.tenant_id();
        let action_results = serde_json::to_value(result.action_results())
            .map_err(HookExecutionLogError::persistence_failed)?;
        let new_row = NewHookExecutionRow {
            id: result.execution_id().into_inner(),
            tenant_id: tenant_id.into_inner(),
            trigger_context_id: result.trigger_context_id().into_inner(),
            hook_id: result.hook_id().as_str().to_owned(),
            trigger_type: result.trigger_type().as_str().to_owned(),
            predicate_data: result.predicate_data().clone(),
            action_results,
            status: result.status().as_str().to_owned(),
            executed_at: result.executed_at(),
        };

        self.execute_query(tenant_id, move |connection| {
            diesel::insert_into(hook_executions::table)
                .values(&new_row)
                .execute(connection)
                .map_err(HookExecutionLogError::persistence_failed)?;
            Ok(())
        })
        .await
    }

    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookExecutionLogResult<Vec<HookExecutionResult>> {
        let tenant_id = ctx.tenant_id();
        let trigger_context_uuid = trigger_context_id.into_inner();
        self.execute_query(tenant_id, move |connection| {
            let rows = hook_executions::table
                // Defense-in-depth: explicit tenant_id filter even though with_tenant_tx
                // sets app.tenant_id for RLS. Retains multi-tenant isolation if RLS is
                // not configured on the table.
                .filter(hook_executions::tenant_id.eq(tenant_id.into_inner()))
                .filter(hook_executions::trigger_context_id.eq(trigger_context_uuid))
                .order_by((
                    hook_executions::executed_at.asc(),
                    hook_executions::hook_id.asc(),
                ))
                .select(HookExecutionRow::as_select())
                .load::<HookExecutionRow>(connection)
                .map_err(HookExecutionLogError::persistence_failed)?;
            rows.into_iter().map(row_to_execution).collect()
        })
        .await
    }
}

fn row_to_execution(row: HookExecutionRow) -> HookExecutionLogResult<HookExecutionResult> {
    let hook_id = HookId::new(row.hook_id)
        .map_err(|err| HookExecutionLogError::invalid_persisted_data(err.to_string()))?;
    let trigger_type = HookTriggerType::try_from(row.trigger_type.as_str())
        .map_err(|err| HookExecutionLogError::invalid_persisted_data(err.to_string()))?;
    let status = HookExecutionStatus::try_from(row.status.as_str())
        .map_err(|err| HookExecutionLogError::invalid_persisted_data(err.to_string()))?;
    let action_results: Vec<ActionResult> = serde_json::from_value(row.action_results)
        .map_err(|err| HookExecutionLogError::invalid_persisted_data(err.to_string()))?;

    Ok(HookExecutionResult::from_persisted(
        HookExecutionPersisted {
            execution_id: HookExecutionId::from_uuid(row.id),
            hook_id,
            trigger_context_id: TriggerContextId::from_uuid(row.trigger_context_id),
            trigger_type,
            predicate_data: row.predicate_data,
            action_results,
            status,
            executed_at: row.executed_at,
        },
    ))
}
