//! Shared world state for hook engine execution BDD scenarios.

use std::sync::Arc;

use corbusier::context::RequestContext;
use corbusier::hook_engine::adapters::memory::{
    InMemoryHookActionExecutor, InMemoryHookDefinitionRepository,
    InMemoryHookExecutionLogRepository, InMemoryHookPolicyAuditRepository,
};
use corbusier::hook_engine::domain::{HookExecutionResult, HookTriggerContext};
use corbusier::hook_engine::services::{HookEngineService, HookEngineServiceDeps};
use corbusier::test_support::test_request_ctx;
use mockable::DefaultClock;
use rstest::fixture;

/// Service type used by the BDD world.
pub type TestHookEngineService = HookEngineService<
    InMemoryHookDefinitionRepository,
    InMemoryHookActionExecutor,
    InMemoryHookExecutionLogRepository,
    InMemoryHookPolicyAuditRepository,
    DefaultClock,
>;

/// Scenario world for hook execution behaviour tests.
pub struct HookWorld {
    /// The hook engine service under test.
    pub service: TestHookEngineService,
    /// Definition repository for configured hooks.
    pub definition_repo: InMemoryHookDefinitionRepository,
    /// Action executor for simulated outcomes.
    pub action_executor: InMemoryHookActionExecutor,
    /// Execution log repository.
    pub execution_log: InMemoryHookExecutionLogRepository,
    /// Policy audit repository.
    pub policy_audit: InMemoryHookPolicyAuditRepository,
    /// Last trigger context used for execution.
    pub last_context: Option<HookTriggerContext>,
    /// Last execution results.
    pub last_results: Option<Vec<HookExecutionResult>>,
    /// Request context used by the scenario.
    pub request_ctx: RequestContext,
}

impl HookWorld {
    /// Creates a world with empty scenario state.
    #[must_use]
    pub fn new() -> Self {
        let definition_repo = InMemoryHookDefinitionRepository::new();
        let action_executor = InMemoryHookActionExecutor::new();
        let execution_log = InMemoryHookExecutionLogRepository::new();
        let policy_audit = InMemoryHookPolicyAuditRepository::new();
        let service = HookEngineService::new(HookEngineServiceDeps {
            definition_repository: Arc::new(definition_repo.clone()),
            action_executor: Arc::new(action_executor.clone()),
            execution_log: Arc::new(execution_log.clone()),
            policy_audit_repository: Arc::new(policy_audit.clone()),
            clock: Arc::new(DefaultClock),
        });
        Self {
            service,
            definition_repo,
            action_executor,
            execution_log,
            policy_audit,
            last_context: None,
            last_results: None,
            request_ctx: test_request_ctx(),
        }
    }
}

impl Default for HookWorld {
    #[rustfmt::skip]
    fn default() -> Self { Self::new() }
}

/// Fixture that creates a new scenario world.
#[fixture]
pub fn world() -> HookWorld {
    HookWorld::default()
}

/// Runs an async operation within sync step definitions.
///
/// # Preconditions
///
/// This function requires that the current thread is running inside a Tokio
/// multi-threaded runtime. It uses [`tokio::task::block_in_place`] internally,
/// which will **panic** if called outside such a runtime or from a current-thread
/// runtime. Ensure that the calling context is within a `#[tokio::test(flavor = "multi_thread")]`
/// or equivalent multi-threaded async runtime environment.
///
/// # Panics
///
/// Panics if called outside a multi-threaded Tokio runtime, as
/// [`tokio::task::block_in_place`] and [`tokio::runtime::Handle::current`]
/// require an active runtime context.
pub fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}
