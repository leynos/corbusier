//! `PostgreSQL` repository implementation for hook execution logs.

use super::models::{HookExecutionRow, NewHookExecutionRow};
use super::schema::hook_executions;
use crate::hook_engine::domain::{
    ActionResult, HookExecutionId, HookExecutionPersisted, HookExecutionResult,
    HookExecutionStatus, HookId, HookTriggerType, TriggerContextId,
};
use crate::hook_engine::ports::{
    HookExecutionLogError, HookExecutionLogRepository, HookExecutionLogResult,
};
use async_trait::async_trait;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};

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

    async fn run_blocking<F, T>(&self, f: F) -> HookExecutionLogResult<T>
    where
        F: FnOnce(&mut PgConnection) -> HookExecutionLogResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut connection = pool.get().map_err(HookExecutionLogError::persistence)?;
            f(&mut connection)
        })
        .await
        .map_err(HookExecutionLogError::persistence)?
    }
}

#[async_trait]
impl HookExecutionLogRepository for PostgresHookExecutionLogRepository {
    async fn store(&self, result: &HookExecutionResult) -> HookExecutionLogResult<()> {
        let action_results = serde_json::to_value(result.action_results())
            .map_err(HookExecutionLogError::persistence)?;
        let new_row = NewHookExecutionRow {
            id: result.execution_id().into_inner(),
            trigger_context_id: result.trigger_context_id().into_inner(),
            hook_id: result.hook_id().as_str().to_owned(),
            trigger_type: result.trigger_type().as_str().to_owned(),
            predicate_data: result.predicate_data().clone(),
            action_results,
            status: result.status().as_str().to_owned(),
            executed_at: result.executed_at(),
        };

        self.run_blocking(move |connection| {
            diesel::insert_into(hook_executions::table)
                .values(&new_row)
                .execute(connection)
                .map_err(HookExecutionLogError::persistence)?;
            Ok(())
        })
        .await
    }

    async fn find_by_trigger_context(
        &self,
        trigger_context_id: TriggerContextId,
    ) -> HookExecutionLogResult<Vec<HookExecutionResult>> {
        let trigger_context_uuid = trigger_context_id.into_inner();
        self.run_blocking(move |connection| {
            let rows = hook_executions::table
                .filter(hook_executions::trigger_context_id.eq(trigger_context_uuid))
                .order_by((
                    hook_executions::executed_at.asc(),
                    hook_executions::hook_id.asc(),
                ))
                .select(HookExecutionRow::as_select())
                .load::<HookExecutionRow>(connection)
                .map_err(HookExecutionLogError::persistence)?;
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
