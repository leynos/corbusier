# Add hook policy enforcement and audit capture (Roadmap 2.3.2)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DONE

This document plans roadmap item 2.3.2 in `docs/roadmap.md`:

- Implement policy evaluation at enforcement points.
- Persist policy violations and audit events.
- Make policy enforcement outcomes queryable by task, conversation, and hook
  event.

The term "hook event" in this plan means a single trigger occurrence identified
by `TriggerContextId`; queries should also support filtering by
`HookTriggerType` when the caller wants an event family rather than one event
instance.

## Purpose / big picture

After this change, Corbusier will not merely execute governance hooks; it will
enforce policy decisions at real workflow boundaries and keep a durable,
queryable audit trail of what was evaluated, what was denied or permitted, and
which task, conversation, and hook event the decision belonged to.

The first concrete enforcement point in the current codebase is tool execution.
A pre-tool-use policy hook can deny a tool call before the MCP host runs it,
and both the denial and the underlying hook action results are persisted. A
post-tool-use hook can record the resulting audit event after the call
completes. The hook audit query surface then allows callers to retrieve policy
outcomes by `TaskId`, by `ConversationId`, or by `TriggerContextId`.

Observable operator outcome:

1. A tool call with a denying pre-tool-use policy is rejected before execution.
2. The rejection is persisted as both a hook execution result and a policy
   audit event.
3. The persisted policy audit event can be retrieved by task, conversation, and
   hook event in unit, in-memory integration, and PostgreSQL-backed tests.

## Constraints

- Preserve hexagonal boundaries across `src/hook_engine/` and
  `src/tool_registry/`.
  - Domain code must stay free of Diesel, process-host, and object-store
    concerns.
  - Ports must be owned by the consuming bounded context.
  - Adapters may depend on other bounded contexts only when implementing a
    local port; adapters must not call one another directly.
- Respect dependency ordering from `docs/roadmap.md`: 2.3.2 builds on the
  existing 2.3.1 hook engine and must not silently widen scope into unfinished
  VCS or HTTP API milestones.
- Keep `RequestContext` focused on tenant and identity concerns. Do not overload
  it with task or conversation references; introduce a separate execution-scope
  value object for workflow correlation.
- Satisfy the roadmap success criterion with indexed, first-class query
  columns. Queryability by task, conversation, and hook event must not depend
  on scanning arbitrary JSON metadata.
- Preserve the existing 2.1.2 tool-call audit trail in
  `tool_call_audit_log`. Policy audit capture augments that trail; it does not
  replace or weaken it.
- Use `rstest` for unit and integration fixtures and parameterized cases, and
  use `rstest-bdd` for behavioural scenarios that exercise user-visible
  workflow behaviour.
- Use the existing `pg-embedded-setup-unpriv` test harness pattern for
  PostgreSQL-backed coverage.
- Keep files under the repository 400-line limit by splitting new domain,
  port, adapter, and test code into focused modules.
- Update documentation as part of the feature:
  - `docs/corbusier-design.md` must record any design decisions taken.
  - `docs/users-guide.md` must describe new hook-backed policy behaviour that a
    user or operator needs to know.
  - `docs/roadmap.md` must mark 2.3.2 done only after all quality gates pass.
- Do not add a new external crate without explicit escalation.

## Tolerances (exception triggers)

- Scope: stop and escalate if implementation exceeds 35 files changed or 2,500
  net new lines.
- Interface: stop and escalate if satisfying 2.3.2 requires incompatible public
  API changes outside `hook_engine` and `tool_registry`.
- Dependency: stop and escalate if a new external crate is required.
- Schema: stop and escalate if the design cannot be satisfied with one additive
  migration touching `hook_executions` plus one new policy-audit projection
  table.
- Iteration: stop and escalate if the same failing test or lint issue persists
  after four focused fix-and-rerun attempts.
- Ambiguity: stop and escalate if the implementation must choose between
  "tool-only enforcement for now" and "partial stubs for future VCS/turn
  enforcement points" without a clearly superior path.

## Risks

- Risk: Current tool execution requests do not carry task or conversation
  identifiers. Severity: high Likelihood: high Mitigation: add a dedicated
  execution-scope value object to tool call inputs and hook trigger contexts,
  keeping `RequestContext` identity-only.

- Risk: Hook execution persistence alone is too coarse to satisfy the query
  requirements because `action_results` are stored as JSON arrays. Severity:
  high Likelihood: high Mitigation: add a dedicated policy-audit projection
  with indexed task, conversation, and trigger-context columns.

- Risk: Direct `tool_registry` dependency on `hook_engine` service types would
  erode bounded-context ownership. Severity: medium Likelihood: medium
  Mitigation: define the governance dependency as a `tool_registry` port and
  implement it with a hook-engine-backed adapter.

- Risk: Behavioural tests can silently fail to compile if step directories are
  not wired through a top-level scenario entry point. Severity: medium
  Likelihood: medium Mitigation: follow the existing `*_scenarios.rs` pattern
  and keep the feature file, step module directory, and entry-point file
  distinct.

- Risk: PostgreSQL integration tests can reuse a stale template database after
  a migration is added. Severity: medium Likelihood: high Mitigation: bump
  `tests/postgres/helpers.rs:TEMPLATE_DB` when adding the migration and keep
  the migration list in chronological order.

## Progress

- [x] (2026-03-21 00:00Z) Gathered roadmap, design, testing, and architecture
      context; drafted this ExecPlan.
- [x] Stage A: finalize the domain model for enforcement scope and policy audit
      projection.
- [x] Stage B: implement hook-engine policy audit persistence and query
      service, including PostgreSQL migration.
- [x] Stage C: wire the first enforcement point into tool execution without
      violating hexagonal boundaries.
- [x] Stage D: add `rstest`, in-memory integration, PostgreSQL integration, and
      `rstest-bdd` behavioural tests.
- [x] Stage E: update design and user documentation; mark roadmap 2.3.2 done.
- [ ] Stage F: run formatting, lint, test, Markdown, and Mermaid validation
      gates and capture evidence.

## Surprises & discoveries

- Observation: `src/hook_engine/domain/trigger.rs` currently stores only
  `trigger_type`, free-form `metadata`, and `occurred_at`. Evidence:
  `HookTriggerContext` has no typed task or conversation references. Impact:
  2.3.2 cannot satisfy its queryability criterion unless the trigger model
  gains a typed execution scope.

- Observation: `src/tool_registry/domain/routing.rs` currently models
  `ToolCallRequest` with `call_id`, `tool_name`, `parameters`, and
  `initiated_at`, but no task or conversation linkage. Evidence: the request
  constructor and accessors expose no workflow correlation fields. Impact: tool
  execution is the first realistic enforcement point, but the request model
  must be widened before policy audit rows can be queried by task or
  conversation.

- Observation: `src/tool_registry/ports/policy.rs` only supports a pre-call
  decision and cannot capture post-call hook audit events. Evidence:
  `ToolPolicyEnforcer::evaluate()` returns only `PolicyDecision`. Impact: the
  tool-governance port must become richer, or a sibling observer port must be
  added, before post-tool-use audit capture can be wired cleanly.

- Observation: `pg-embedded` startup in this environment still depends on a
  correct `/dev/null` character device. Evidence: the first PostgreSQL rerun
  failed with `cannot create /dev/null: Permission denied` until `/dev/null`
  was recreated as `c 1 3`. Impact: Postgres validation remains environment
  sensitive even when the application code is correct.

## Decision log

- Decision: keep `RequestContext` identity-scoped and add a separate execution
  scope for task and conversation correlation. Rationale: tenant, user,
  session, and tracing identity are cross-cutting concerns, while task and
  conversation linkage are workflow concerns. Mixing them would make
  `RequestContext` a grab-bag and leak business context into unrelated
  adapters. Date/Author: 2026-03-21 / plan author.

- Decision: satisfy the roadmap queryability requirement with a dedicated
  policy-audit projection table and query service rather than querying
  `hook_executions.action_results` JSON directly. Rationale: the success
  criterion requires stable, indexed queries by task, conversation, and hook
  event. A projection keeps the hot query path simple, additive, and
  adapter-friendly while preserving the richer execution payload in
  `hook_executions`. Date/Author: 2026-03-21 / plan author.

- Decision: expose hook-backed enforcement to the tool plane through a
  `tool_registry`-owned governance port, implemented by an adapter that
  delegates to `hook_engine`. Rationale: the consuming bounded context must own
  the dependency contract. This preserves hexagonal boundaries and prevents
  `tool_registry` domain or service code from depending directly on
  `hook_engine` internals. Date/Author: 2026-03-21 / plan author.

- Decision: implement only the currently reachable service-layer and
  tool-execution-layer enforcement points in this milestone; do not stub
  incomplete VCS or HTTP middleware enforcement. Rationale: the design document
  lists multiple layers, but the current repository only exposes tool execution
  as a concrete, testable enforcement path. Shipping one end-to-end enforcement
  point with durable audit data is better than adding speculative scaffolding
  for absent modules. Date/Author: 2026-03-21 / plan author.

## Outcomes & retrospective

Implemented outcome:

- `hook_engine` owns a queryable policy-audit model and persistence port.
- `tool_registry` enforces pre-tool-use policy and records post-tool-use hook
  audit through a local governance port.
- Unit, in-memory integration, PostgreSQL integration, and behavioural tests
  prove happy paths, unhappy paths, and key edge cases.
- `docs/corbusier-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`
  accurately reflect the implemented behaviour.

Retrospective:

- The dedicated execution-scope objects kept workflow correlation out of
  `RequestContext` while still making policy audit rows queryable by task and
  conversation.
- A separate policy-audit projection table was the right trade-off; querying
  indexed rows is materially simpler than inspecting `hook_executions`
  `action_results` JSON.
- The environment-level `/dev/null` failure remains a sharp edge for
  PostgreSQL-backed tests and was handled as an infrastructure repair rather
  than a code change.

## Context and orientation

The current repository baseline already includes the 2.3.1 hook engine:

- `src/hook_engine/services/engine.rs` executes configured hooks, stores
  pending execution rows, and persists the final `HookExecutionResult`.
- `src/hook_engine/ports/execution_log.rs` exposes persistence only for hook
  execution results keyed by `TriggerContextId`.
- `src/hook_engine/adapters/memory/execution_log.rs` and
  `src/hook_engine/adapters/postgres/repository.rs` implement that execution
  log port.

The tool plane already includes one pre-execution policy seam:

- `src/tool_registry/services/discovery/mod.rs` resolves tools, validates input
  schema, calls `ToolPolicyEnforcer`, invokes the MCP host, and writes the
  existing tool-call audit trail.
- `src/tool_registry/ports/policy.rs` defines the current pre-call
  `ToolPolicyEnforcer` contract.
- `src/tool_registry/adapters/policy.rs` currently provides simple allow-all,
  deny-all, and fail-fast adapters for tests and default wiring.

The missing pieces for 2.3.2 are therefore:

1. A typed way to carry task and conversation correlation through hook-trigger
   execution.
2. A hook-owned policy-audit persistence and query surface.
3. A tool-plane governance abstraction that can delegate to the hook engine
   without crossing bounded-context ownership lines.
4. Documentation and tests that prove the first real enforcement point works
   end to end.

The current PostgreSQL test harness centralizes schema setup in
`tests/postgres/helpers.rs`. The file is already versioned at
`corbusier_test_template_v10`; any new migration added for this feature must
also bump that suffix to avoid stale template reuse.

## Plan of work

### Stage A: model the enforcement scope and policy audit shape

Add the typed workflow-correlation model that 2.3.2 requires.

In `src/hook_engine/domain/trigger.rs`, introduce an additive execution-scope
value object, for example `HookExecutionScope`, carrying:

- `task_id: Option<crate::task::domain::TaskId>`
- `conversation_id: Option<crate::message::domain::ConversationId>`
- `metadata: serde_json::Value`

Update `HookTriggerContext` to store this scope instead of raw metadata-only
state, while retaining a free-form metadata payload inside the scope for
non-indexed details such as tool name or policy configuration echoes.

In `src/hook_engine/domain/`, add a focused policy-audit module that defines:

- `PolicyAuditEventId`
- `PolicyAuditDecision` (`allow`, `deny`, and, if needed, `conditional`)
- `PolicyViolation` with stable machine-readable code plus human-readable
  reason
- `PolicyAuditEvent` containing hook execution identifiers, action identifiers,
  task and conversation references, hook event identity, decision, optional
  violation payload, and timestamp

Also add a domain helper that normalizes a `PolicyCheck` action result into a
`PolicyAuditEvent`. Keep this parsing or projection logic in the hook domain so
service code does not inspect raw JSON ad hoc.

Validation gate for Stage A:

- Add or update unit tests in `src/hook_engine/tests/domain_tests.rs` proving
  execution-scope construction, policy-audit normalization, and invalid policy
  output handling.
- Do not proceed until the new domain model compiles cleanly and the added unit
  tests fail before the implementation and pass after it.

### Stage B: add hook-engine policy-audit persistence and query services

In `src/hook_engine/ports/`, add a `policy_audit.rs` port describing the
hook-owned persistence/query contract, for example:

```rust
#[async_trait::async_trait]
pub trait HookPolicyAuditRepository: Send + Sync {
    async fn store(
        &self,
        ctx: &RequestContext,
        event: &PolicyAuditEvent,
    ) -> HookPolicyAuditResult<()>;

    async fn find_by_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;

    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;
}
```

Add a small query-facing service in `src/hook_engine/services/`, for example
`HookPolicyAuditQueryService`, so future callers do not need to talk to the
repository directly.

Update `src/hook_engine/services/engine.rs` so that after executing a hook
definition it:

1. Builds the `HookExecutionResult` as today.
2. Extracts zero or more `PolicyAuditEvent` projections from its action
   results.
3. Stores the hook execution result.
4. Stores the derived policy audit events.

Keep the order explicit: if execution persistence succeeds but policy-audit
persistence fails, bubble a typed error and document that behaviour in the
decision log when implementing.

In `src/hook_engine/adapters/memory/`, add an in-memory policy-audit repository
keyed by tenant and indexed in-memory by task, conversation, and trigger
context.

In `src/hook_engine/adapters/postgres/`, add a dedicated adapter and migration
for a new projection table, for example `hook_policy_audit_events`, with at
least these columns:

- `id UUID PRIMARY KEY`
- `tenant_id UUID NOT NULL`
- `hook_execution_id UUID NOT NULL`
- `trigger_context_id UUID NOT NULL`
- `trigger_type VARCHAR NOT NULL`
- `hook_id TEXT NOT NULL`
- `action_id TEXT NOT NULL`
- `task_id UUID NULL`
- `conversation_id UUID NULL`
- `decision VARCHAR NOT NULL`
- `violation JSONB NULL`
- `payload JSONB NOT NULL`
- `recorded_at TIMESTAMPTZ NOT NULL`

Use indexed columns for the required query shapes:

- `(tenant_id, task_id, recorded_at)`
- `(tenant_id, conversation_id, recorded_at)`
- `(tenant_id, trigger_context_id, recorded_at)`

Prefer composite tenant-consistency constraints. If the new table references
`hook_executions`, add the supporting uniqueness on `(id, tenant_id)` to
`hook_executions` inside the same migration so the foreign key can remain
tenant-safe.

Update `tests/postgres/helpers.rs` with the new migration constant, wire it
into `apply_migrations()` in chronological order, and bump `TEMPLATE_DB`.

Validation gate for Stage B:

- Add in-memory integration tests proving policy audit events are persisted and
  queryable by task, conversation, and trigger context.
- Add PostgreSQL integration tests proving the same queries work under
  tenant-scoped storage and that cross-tenant lookups return empty results.
- Add a duplicate-trigger-context test to prove idempotent hook execution does
  not duplicate policy audit projections.

### Stage C: wire the first enforcement point into tool execution

Add a local governance abstraction to `tool_registry` instead of calling the
hook engine directly from `ToolDiscoveryRoutingService`.

In `src/tool_registry/ports/`, replace or supersede the current policy-only
port with a richer governance contract owned by the tool plane, for example:

```rust
#[async_trait::async_trait]
pub trait ToolExecutionGovernance: Send + Sync {
    async fn enforce_before_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
    ) -> ToolGovernanceResult<ToolGovernanceDecision>;

    async fn observe_after_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
        result: &ToolCallResult,
    ) -> ToolGovernanceResult<()>;
}
```

Add a tool-execution scope to `src/tool_registry/domain/routing.rs` so
`ToolCallRequest` can carry task and conversation references needed by the hook
audit projection. Make the new scope additive by providing builder helpers such
as `with_task_id(...)`, `with_conversation_id(...)`, or
`with_execution_scope(...)`.

Implement a hook-backed adapter for the new port. The adapter should:

1. Translate the tool call plus execution scope into a `HookTriggerContext`
   with `HookTriggerType::PreToolUse` or `HookTriggerType::PostToolUse`.
2. Delegate execution to the `hook_engine` port.
3. Interpret hook results to derive allow or deny for the pre-call case.
4. Let the hook engine persist both execution results and policy audit events.

Keep the existing trivial allow-all and deny-all behaviour available through
simple adapter implementations for narrow tests.

Update `src/tool_registry/services/discovery/mod.rs` so the enforcement flow is
explicit:

1. Resolve tool and validate schema.
2. Run `enforce_before_call(...)`.
3. If denied, emit the existing tool-call rejection audit and return a policy
   denial error before invoking the host.
4. If permitted, call the MCP host.
5. Persist the existing tool-call audit trail.
6. Run `observe_after_call(...)` to capture the post-tool-use hook event.

This stage is complete only when the policy denial actually blocks the tool
call and both the tool-call audit and hook-policy audit are persisted.

Validation gate for Stage C:

- Add or extend `rstest` unit tests for the governance adapter and discovery
  service denial path.
- Add in-memory integration tests proving a denied pre-tool-use hook blocks the
  host invocation and still leaves queryable hook-policy audit rows.
- Add PostgreSQL-backed routing tests proving the same behaviour with the new
  policy-audit projection table.

### Stage D: add behavioural tests and finish documentation

Add a dedicated BDD feature for user-observable policy enforcement. Keep it in
its own files to avoid the step-module compilation trap:

- `tests/features/hook_policy_enforcement.feature`
- `tests/hook_policy_enforcement_scenarios.rs`
- `tests/hook_policy_enforcement_steps/{mod.rs,world.rs,given.rs,when.rs,then.rs}`

Cover at least these scenarios:

1. A pre-tool-use policy permits a tool call and the policy audit can be
   retrieved by conversation.
2. A pre-tool-use policy denies a tool call and the denial can be retrieved by
   task.
3. A post-tool-use hook records an audit event retrievable by hook event.

Update `docs/corbusier-design.md` to record the implementation decisions taken
in this milestone. At minimum, document:

- the introduction of a typed hook execution scope,
- the dedicated policy-audit projection/query model,
- and the fact that 2.3.2 implements the tool-execution enforcement point
  first, leaving future API/VCS enforcement points for their owning roadmap
  items.

Update `docs/users-guide.md` in the existing tool-routing section to explain:

- that tool calls now pass through hook-backed governance before execution,
- that denial outcomes can come from configured pre-tool-use policies,
- and that policy audit outcomes are queryable by task, conversation, and hook
  event through the hook audit service.

Only after those docs and all gates pass should `docs/roadmap.md` mark 2.3.2
and its sub-bullets done.

## Concrete steps

Run all commands from `/home/user/project`.

1. Read the relevant code and add failing tests first.

   ```bash
   cargo test --workspace hook_engine::tests::domain_tests -- --nocapture
   ```

   Expected shape:

   ```plaintext
   running ... tests
   test ... fails because policy audit support is not implemented yet
   ```

2. Implement the hook-engine domain and audit repository changes, then run the
   focused tests.

   ```bash
   cargo test --workspace hook_engine -- --nocapture
   ```

   Expected shape:

   ```plaintext
   running ... tests
   test hook_engine::... ... ok
   ```

3. Add and verify in-memory integration coverage.

   ```bash
   cargo test --workspace in_memory::hook_engine_tests -- --nocapture
   cargo test --workspace in_memory::tool_discovery_routing_tests -- --nocapture
   ```

4. Add and verify PostgreSQL-backed coverage with the embedded cluster
   harness.

   ```bash
   cargo install pg-embedded-setup-unpriv
   cargo test --workspace postgres::hook_engine_tests -- --nocapture
   cargo test --workspace postgres::tool_discovery_routing_tests -- --nocapture
   ```

5. Add behavioural coverage.

   ```bash
   cargo test --workspace hook_policy_enforcement_scenarios -- --nocapture
   ```

6. Run the required repository quality gates with logs captured via `tee`.

   ```bash
   set -o pipefail; make check-fmt 2>&1 | tee /tmp/2-3-2-check-fmt.log
   set -o pipefail; make lint 2>&1 | tee /tmp/2-3-2-lint.log
   set -o pipefail; make test TEST_FLAGS='--profile long --all-targets --all-features' 2>&1 | tee /tmp/2-3-2-test.log
   set -o pipefail; make fmt 2>&1 | tee /tmp/2-3-2-fmt.log
   set -o pipefail; PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/2-3-2-markdownlint.log
   set -o pipefail; make nixie 2>&1 | tee /tmp/2-3-2-nixie.log
   ```

   Expected shape:

   ```plaintext
   ... finished with status: success
   ... test result: ok
   ... markdownlint: 0 errors
   ... All diagrams validated successfully
   ```

## Validation and acceptance

Acceptance is behavioural, not structural.

The feature is done when all of the following are true:

- A configured denying pre-tool-use hook causes
  `ToolDiscoveryRoutingService::call_tool()` to reject the call before the host
  executes the tool.
- The denial still writes the existing tool-call rejection audit row.
- The same denial also writes a hook-policy audit event that is retrievable by
  `TaskId`, by `ConversationId`, and by `TriggerContextId`.
- A permitted tool call still succeeds, and a post-tool-use audit event is
  persisted after execution.
- In-memory and PostgreSQL-backed tests prove tenant isolation for those audit
  queries.
- Behavioural tests demonstrate the workflow from the caller's perspective.

Quality criteria:

- Tests: new `rstest`, in-memory integration, PostgreSQL integration, and
  `rstest-bdd` scenarios pass.
- Lint and formatting: `make check-fmt`, `make lint`, and `make fmt` pass.
- Documentation: `make markdownlint` and `make nixie` pass after updating the
  design doc, user guide, and roadmap.

## Idempotence and recovery

The implementation steps in this plan are additive and should be safely
repeatable.

- Re-running the new migration in a fresh test database is safe because the
  template database version must be bumped when the migration list changes.
- Re-running focused tests is safe; use them as the first retry path after any
  failure.
- If `make fmt` rewrites unrelated Markdown files, restore the unrelated churn
  before finalizing and keep only the intended documentation changes.
- If PostgreSQL tests appear to use an old schema, verify that
  `tests/postgres/helpers.rs:TEMPLATE_DB` was bumped and that the new migration
  was added to `apply_migrations()` in order.

## Artifacts and notes

The most important evidence to capture during implementation is the query
surface itself. Keep short proof points such as:

```plaintext
policy audit query by task returns 1 event for denied tool call
policy audit query by conversation returns 1 event for permitted tool call
policy audit query by trigger_context returns the exact hook event just executed
```

For the PostgreSQL adapter, a concise raw-SQL verification is appropriate in
integration tests, for example:

```sql
SELECT count(*)
FROM hook_policy_audit_events
WHERE tenant_id = $1
  AND task_id = $2;
```

## Interfaces and dependencies

The implementation should end with these stable repository-relative interfaces
or their close equivalents.

In `src/hook_engine/domain/trigger.rs`, define an additive execution-scope
model owned by the hook engine.

```rust
pub struct HookExecutionScope {
    pub task_id: Option<TaskId>,
    pub conversation_id: Option<ConversationId>,
    pub metadata: serde_json::Value,
}
```

In `src/hook_engine/ports/policy_audit.rs`, define the hook-owned audit query
contract.

```rust
#[async_trait::async_trait]
pub trait HookPolicyAuditRepository: Send + Sync {
    async fn store(
        &self,
        ctx: &RequestContext,
        event: &PolicyAuditEvent,
    ) -> HookPolicyAuditResult<()>;

    async fn find_by_task(
        &self,
        ctx: &RequestContext,
        task_id: TaskId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;

    async fn find_by_conversation(
        &self,
        ctx: &RequestContext,
        conversation_id: ConversationId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;

    async fn find_by_trigger_context(
        &self,
        ctx: &RequestContext,
        trigger_context_id: TriggerContextId,
    ) -> HookPolicyAuditResult<Vec<PolicyAuditEvent>>;
}
```

In `src/tool_registry/ports/`, define a tool-plane-owned governance contract
that hides hook-engine details from the service layer.

```rust
#[async_trait::async_trait]
pub trait ToolExecutionGovernance: Send + Sync {
    async fn enforce_before_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
    ) -> ToolGovernanceResult<ToolGovernanceDecision>;

    async fn observe_after_call(
        &self,
        ctx: &RequestContext,
        request: &ToolCallRequest,
        entry: &CatalogEntry,
        result: &ToolCallResult,
    ) -> ToolGovernanceResult<()>;
}
```

In `src/tool_registry/domain/routing.rs`, keep `ToolCallRequest` additive and
introduce a workflow-correlation scope rather than mutating `RequestContext`.

```rust
pub struct ToolExecutionScope {
    pub task_id: Option<TaskId>,
    pub conversation_id: Option<ConversationId>,
    pub metadata: serde_json::Value,
}
```

The only infrastructure dependencies needed are the existing Diesel,
PostgreSQL, `mockable`, `rstest`, `rstest-bdd`, and `pg-embedded-setup-unpriv`
tooling already present in the repository.

## Revision note

Revision 0 (2026-03-21): initial draft created. The plan fixes the scope on the
first real enforcement point (tool execution), introduces a dedicated
policy-audit projection/query model, and records that task and conversation
queryability requires a new execution-scope value object instead of widening
`RequestContext`.
