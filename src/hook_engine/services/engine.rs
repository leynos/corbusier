//! Hook engine orchestration service.

use crate::context::RequestContext;
use crate::hook_engine::domain::{
    ActionResult, HookDefinition, HookExecutionId, HookExecutionInput, HookExecutionResult,
    HookTriggerContext, HookTriggerType, project_policy_audit_events,
};
use crate::hook_engine::ports::{
    HookActionExecutor, HookDefinitionRepository, HookEngine, HookEngineResult,
    HookExecutionLogError, HookExecutionLogRepository, HookPolicyAuditRepository,
    PendingExecutionRecord, PendingExecutionReservation,
};
use mockable::Clock;
use std::sync::Arc;

const SUPPORTED_TRIGGERS: [HookTriggerType; 14] = HookTriggerType::all();

/// The request, definition and trigger one hook execution runs against.
#[derive(Clone, Copy)]
struct Invocation<'a> {
    ctx: &'a RequestContext,
    definition: &'a HookDefinition,
    context: &'a HookTriggerContext,
}

/// Hook execution orchestration service.
#[derive(Clone)]
pub struct HookEngineService<D, A, L, P, C>
where
    D: HookDefinitionRepository,
    A: HookActionExecutor,
    L: HookExecutionLogRepository,
    P: HookPolicyAuditRepository,
    C: Clock + Send + Sync,
{
    definition_repository: Arc<D>,
    action_executor: Arc<A>,
    execution_log: Arc<L>,
    policy_audit_repository: Arc<P>,
    clock: Arc<C>,
}

/// Dependencies for constructing a [`HookEngineService`].
pub struct HookEngineServiceDeps<D, A, L, P, C>
where
    D: HookDefinitionRepository,
    A: HookActionExecutor,
    L: HookExecutionLogRepository,
    P: HookPolicyAuditRepository,
    C: Clock + Send + Sync,
{
    /// Repository for retrieving hook definitions.
    pub definition_repository: Arc<D>,
    /// Executor for running hook actions.
    pub action_executor: Arc<A>,
    /// Repository for persisting execution logs.
    pub execution_log: Arc<L>,
    /// Repository for storing policy audit events.
    pub policy_audit_repository: Arc<P>,
    /// Clock for timestamping execution records.
    pub clock: Arc<C>,
}

impl<D, A, L, P, C> HookEngineService<D, A, L, P, C>
where
    D: HookDefinitionRepository,
    A: HookActionExecutor,
    L: HookExecutionLogRepository,
    P: HookPolicyAuditRepository,
    C: Clock + Send + Sync,
{
    /// Creates a new hook engine service.
    #[must_use]
    pub fn new(deps: HookEngineServiceDeps<D, A, L, P, C>) -> Self {
        Self {
            definition_repository: deps.definition_repository,
            action_executor: deps.action_executor,
            execution_log: deps.execution_log,
            policy_audit_repository: deps.policy_audit_repository,
            clock: deps.clock,
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

    async fn persist_policy_audit_events(
        &self,
        ctx: &RequestContext,
        result: &HookExecutionResult,
        context: &HookTriggerContext,
    ) -> HookEngineResult<()> {
        for event in project_policy_audit_events(result, context)? {
            self.policy_audit_repository.store(ctx, &event).await?;
        }
        Ok(())
    }

    /// Reserves a pending execution record for a definition and trigger.
    async fn reserve_pending_execution(
        &self,
        call: Invocation<'_>,
        execution_id: HookExecutionId,
    ) -> HookEngineResult<PendingExecutionReservation> {
        let Invocation {
            ctx,
            definition,
            context,
        } = call;
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
        Ok(reservation)
    }

    /// Returns the stored result of an execution another caller reserved,
    /// re-emitting its policy audit events.
    async fn replay_reserved_execution(
        &self,
        call: Invocation<'_>,
        existing_execution_id: HookExecutionId,
    ) -> HookEngineResult<HookExecutionResult> {
        let Invocation {
            ctx,
            definition,
            context,
        } = call;
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
        self.persist_policy_audit_events(ctx, &existing_result, context)
            .await?;
        Ok(existing_result)
    }

    /// Runs a definition's actions, then persists its audit events and result.
    async fn run_definition(
        &self,
        call: Invocation<'_>,
        execution_id: HookExecutionId,
    ) -> HookEngineResult<HookExecutionResult> {
        let Invocation {
            ctx,
            definition,
            context,
        } = call;
        let action_results = self.execute_actions(definition, context).await?;
        let result = HookExecutionResult::new(HookExecutionInput {
            execution_id,
            hook_id: definition.id().clone(),
            trigger_context_id: context.id(),
            trigger_type: context.trigger_type(),
            predicate_data: definition.predicate().data().clone(),
            action_results,
            executed_at: self.clock.utc(),
        });
        self.persist_policy_audit_events(ctx, &result, context)
            .await?;
        self.execution_log.update_result(ctx, &result).await?;
        Ok(result)
    }
}

#[async_trait::async_trait]
impl<D, A, L, P, C> HookEngine for HookEngineService<D, A, L, P, C>
where
    D: HookDefinitionRepository,
    A: HookActionExecutor,
    L: HookExecutionLogRepository,
    P: HookPolicyAuditRepository,
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
            let execution_id = HookExecutionId::new();
            let call = Invocation {
                ctx,
                definition: &definition,
                context: &context,
            };
            let reservation = self.reserve_pending_execution(call, execution_id).await?;
            let result = match reservation {
                PendingExecutionReservation::AlreadyExists(existing_execution_id) => {
                    self.replay_reserved_execution(call, existing_execution_id)
                        .await?
                }
                PendingExecutionReservation::Created(_) => {
                    self.run_definition(call, execution_id).await?
                }
            };
            results.push(result);
        }
        Ok(results)
    }

    fn supported_triggers(&self) -> &'static [HookTriggerType] {
        &SUPPORTED_TRIGGERS
    }
}
