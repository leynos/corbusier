//! Hook engine orchestration service.

use crate::context::RequestContext;
use crate::hook_engine::domain::{
    ActionResult, HookDefinition, HookExecutionInput, HookExecutionResult, HookTriggerContext,
    HookTriggerType,
};
use crate::hook_engine::ports::{
    HookActionExecutor, HookDefinitionRepository, HookEngine, HookEngineResult,
    HookExecutionLogError, HookExecutionLogRepository, PendingExecutionRecord,
    PendingExecutionReservation,
};
use mockable::Clock;
use std::sync::Arc;

const SUPPORTED_TRIGGERS: [HookTriggerType; 14] = HookTriggerType::all();

/// Hook execution orchestration service.
#[derive(Clone)]
pub struct HookEngineService<D, A, L, C>
where
    D: HookDefinitionRepository,
    A: HookActionExecutor,
    L: HookExecutionLogRepository,
    C: Clock + Send + Sync,
{
    definition_repository: Arc<D>,
    action_executor: Arc<A>,
    execution_log: Arc<L>,
    clock: Arc<C>,
}

impl<D, A, L, C> HookEngineService<D, A, L, C>
where
    D: HookDefinitionRepository,
    A: HookActionExecutor,
    L: HookExecutionLogRepository,
    C: Clock + Send + Sync,
{
    /// Creates a new hook engine service.
    ///
    /// Example: `HookEngineService::new(def_repo, executor, log_repo, clock)`
    /// wires the engine to its dependencies.
    #[must_use]
    pub const fn new(
        definition_repository: Arc<D>,
        action_executor: Arc<A>,
        execution_log: Arc<L>,
        clock: Arc<C>,
    ) -> Self {
        Self {
            definition_repository,
            action_executor,
            execution_log,
            clock,
        }
    }

    fn sort_definitions(definitions: &mut [HookDefinition]) {
        definitions.sort_by(|left, right| {
            left.priority()
                .cmp(&right.priority())
                .then_with(|| left.id().cmp(right.id()))
        });
    }

    /// Executes actions in definition order and fails fast on the first
    /// execution error.
    ///
    /// This behaviour is intentional to preserve deterministic failure semantics
    /// for policy hooks, and no execution record is persisted for a definition
    /// when one of its actions fails.
    async fn execute_actions(
        &self,
        definition: &HookDefinition,
        context: &HookTriggerContext,
    ) -> HookEngineResult<Vec<ActionResult>> {
        let mut action_results = Vec::with_capacity(definition.actions().len());
        for action in definition.actions() {
            let result = self.action_executor.execute(action, context).await?;
            action_results.push(result);
        }
        Ok(action_results)
    }
}

#[async_trait::async_trait]
impl<D, A, L, C> HookEngine for HookEngineService<D, A, L, C>
where
    D: HookDefinitionRepository,
    A: HookActionExecutor,
    L: HookExecutionLogRepository,
    C: Clock + Send + Sync,
{
    async fn execute(
        &self,
        ctx: &RequestContext,
        context: HookTriggerContext,
    ) -> HookEngineResult<Vec<HookExecutionResult>> {
        let mut definitions = self
            .definition_repository
            .list_enabled_for_trigger(ctx, context.trigger_type())
            .await?;
        Self::sort_definitions(&mut definitions);

        let mut results = Vec::with_capacity(definitions.len());
        for definition in definitions {
            let execution_id = crate::hook_engine::domain::HookExecutionId::new();
            let reservation = self
                .execution_log
                .store_pending(
                    ctx,
                    PendingExecutionRecord {
                        execution_id,
                        hook_id: definition.id().clone(),
                        trigger_context_id: context.id(),
                        trigger_type: context.trigger_type(),
                        executed_at: self.clock.utc(),
                    },
                )
                .await?;
            if let PendingExecutionReservation::AlreadyExists(existing_execution_id) = reservation {
                let existing_result = self
                    .execution_log
                    .find_by_trigger_context(ctx, context.id())
                    .await?
                    .into_iter()
                    .find(|result| {
                        result.execution_id() == existing_execution_id
                            && result.hook_id() == definition.id()
                    })
                    .ok_or_else(|| {
                        HookExecutionLogError::invalid_persisted_data(format!(
                            "missing reserved hook execution for {existing_execution_id}"
                        ))
                    })?;
                results.push(existing_result);
                continue;
            }
            let action_results = self.execute_actions(&definition, &context).await?;
            let result = HookExecutionResult::new(HookExecutionInput {
                execution_id,
                hook_id: definition.id().clone(),
                trigger_context_id: context.id(),
                trigger_type: context.trigger_type(),
                predicate_data: definition.predicate().data().clone(),
                action_results,
                executed_at: self.clock.utc(),
            });
            self.execution_log.update_result(ctx, &result).await?;
            results.push(result);
        }
        Ok(results)
    }

    fn supported_triggers(&self) -> &'static [HookTriggerType] {
        &SUPPORTED_TRIGGERS
    }
}
