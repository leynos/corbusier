# Add hook policy enforcement and audit capture (Roadmap 2.3.2)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DONE

Companion documents:

- [Interface specification](./2-3-2-hook-policy-enforcement-and-audit-capture-interface-spec.md)
- [Commands and validation checklist](./2-3-2-hook-policy-enforcement-and-audit-capture-commands-and-checklist.md)

This document plans roadmap item 2.3.2 in `docs/roadmap.md`:

- Implement policy evaluation at enforcement points.
- Persist policy violations and audit events.
- Make policy enforcement outcomes queryable by task, conversation, and
  hook event.

The term "hook event" in this plan means a single trigger occurrence identified
by `TriggerContextId`; queries should also support filtering by
`HookTriggerType` when the caller wants an event family rather than one event
instance.

## Purpose / big picture

Corbusier now enforces hook-backed policy decisions at a real workflow boundary
and keeps a durable, queryable audit trail of what was evaluated, what was
denied or permitted, and which task, conversation, and hook event the decision
belonged to.

The first concrete enforcement point is tool execution. A `PreToolCall` policy
hook can deny a tool call before the Model Context Protocol (MCP) host runs it,
and both the denial and the underlying hook action results are persisted. A
`PostToolCall` hook can record the resulting audit event after the call
completes. The audit query surface then allows callers to retrieve policy
outcomes by `TaskId`, `ConversationId`, or `TriggerContextId`.

Observable operator outcome:

1. A tool call with a denying `PreToolCall` policy is rejected before
   execution.
2. The rejection is persisted as both a hook execution result and a policy
   audit event.
3. The persisted policy audit event can be retrieved by task,
   conversation, and hook event in unit, in-memory integration, and
   PostgreSQL-backed tests.

## Constraints

- Preserve hexagonal boundaries across `src/hook_engine/` and
  `src/tool_registry/`.
- Keep `RequestContext` focused on tenant and identity concerns; workflow
  correlation belongs in a dedicated execution-scope value object.
- Satisfy the queryability requirement with indexed columns rather than
  scanning arbitrary JSON metadata.
- Preserve the existing tool-call audit trail; policy audit capture augments
  it.
- Use `rstest` for unit and integration fixtures and `rstest-bdd` for
  behavioural scenarios.
- Use the existing `pg-embedded-setup-unpriv` harness pattern for
  PostgreSQL-backed coverage.
- Keep files under the repository 400-line limit by splitting large modules
  and plans into focused companions.
- Update the design doc, user guide, and roadmap as part of the feature.
- Do not add a new external crate without explicit escalation.

## Tolerances (exception triggers)

- Scope: stop and escalate if implementation exceeds 35 files changed or
  2,500 net new lines.
- Interface: stop and escalate if satisfying 2.3.2 requires incompatible
  public API changes outside `hook_engine` and `tool_registry`.
- Dependency: stop and escalate if a new external crate is required.
- Schema: stop and escalate if the design cannot be satisfied with one
  additive migration touching `hook_executions` plus one new policy-audit
  projection table.
- Iteration: stop and escalate if the same failing test or lint issue
  persists after four focused fix-and-rerun attempts.
- Ambiguity: stop and escalate if the implementation must choose between
  "tool-only enforcement for now" and "partial stubs for future VCS or turn
  enforcement points" without a clearly superior path.

## Risks

- Risk: tool execution requests lacked task and conversation identifiers.
  Mitigation: add a dedicated execution-scope value object to tool call inputs
  and hook trigger contexts.
- Risk: hook execution persistence alone was too coarse to satisfy the query
  requirements because action results were stored as JSON arrays. Mitigation:
  add a dedicated policy-audit projection with indexed task, conversation, and
  trigger-context columns.
- Risk: direct `tool_registry` dependency on `hook_engine` service types
  would erode bounded-context ownership. Mitigation: define the governance
  dependency as a `tool_registry` port and implement it with a
  hook-engine-backed adapter.
- Risk: behavioural tests can silently fail to compile if the step
  directories are not wired through a top-level scenario entry point.
  Mitigation: follow the existing `*_scenarios.rs` pattern.
- Risk: PostgreSQL integration tests can reuse a stale template database
  after a migration is added. Mitigation: bump
  `tests/postgres/helpers.rs:TEMPLATE_DB` whenever the migration list changes.

## Progress

- [x] (2026-03-21 00:00Z) Gathered roadmap, design, testing, and
      architecture context; drafted this ExecPlan.
- [x] Stage A: finalised the domain model for enforcement scope and policy
      audit projection.
- [x] Stage B: implemented hook-engine policy audit persistence and query
      service, including the PostgreSQL migration.
- [x] Stage C: wired the first enforcement point into tool execution without
      violating hexagonal boundaries.
- [x] Stage D: added `rstest`, in-memory integration, PostgreSQL
      integration, and `rstest-bdd` behavioural tests.
- [x] Stage E: updated design and user documentation; marked roadmap 2.3.2
      done.
- [x] Stage F: ran formatting, lint, test, Markdown, and Mermaid validation
      gates; validation details are captured in the companion checklist.

## Surprises & discoveries

- `HookTriggerContext` originally had no typed task or conversation
  references, so 2.3.2 needed a typed execution scope before the query surface
  could exist.
- `ToolCallRequest` originally had no workflow correlation fields, which
  made tool execution impossible to query by task or conversation.
- The earlier policy-only tool port could not support post-call audit
  capture, so the tool plane needed a richer governance contract.
- PostgreSQL validation in this environment remained sensitive to host
  infrastructure details such as a correct `/dev/null` device.

## Decision log

- Decision: keep `RequestContext` identity-scoped and add separate
  execution-scope value objects for workflow correlation.
- Decision: satisfy the queryability requirement with a dedicated
  policy-audit projection table rather than querying
  `hook_executions.action_results` JSON directly.
- Decision: expose hook-backed enforcement through a
  `tool_registry`-owned governance port implemented by an adapter that
  delegates to the hook engine.
- Decision: implement only the currently reachable service-layer and
  tool-execution-layer enforcement points in this milestone rather than
  speculative VCS or HTTP middleware enforcement.

## Plan of work

### Stage A: model the enforcement scope and policy audit shape

- Add typed execution-scope models for hook-trigger and tool-call workflow
  correlation.
- Add the hook-owned policy-audit domain model and projection logic.
- Prove execution-scope construction, policy-audit normalisation, and
  invalid policy output handling with focused unit tests.

### Stage B: add hook-engine policy-audit persistence and query services

- Add a hook-owned policy-audit repository port and query service.
- Persist policy-audit projections alongside hook execution results.
- Add in-memory and PostgreSQL adapters with tenant-scoped query coverage.

### Stage C: wire the first enforcement point into tool execution

- Introduce a tool-plane-owned governance contract that hides hook-engine
  internals from the discovery service.
- Extend `ToolCallRequest` with workflow correlation scope.
- Translate tool calls into hook trigger contexts for `PreToolCall` and
  `PostToolCall` enforcement.
- Ensure denials block host execution while both tool-call and hook-policy
  audit trails remain intact.

### Stage D: add behavioural tests and finish documentation

- Add BDD scenarios for allow, deny, and post-call audit flows.
- Update `docs/corbusier-design.md`, `docs/users-guide.md`, and
  `docs/roadmap.md`.
- Capture stable interfaces and operator commands in the companion docs
  instead of growing this plan past the repository line limit.

## Validation and acceptance

The feature is done when all of the following are true:

- A configured denying `PreToolCall` hook causes
  `ToolDiscoveryRoutingService::call_tool()` to reject the call before the host
  executes the tool.
- The denial still writes the existing tool-call rejection audit row.
- The same denial also writes a hook-policy audit event that is retrievable
  by `TaskId`, `ConversationId`, and `TriggerContextId`.
- A permitted tool call still succeeds, and a `PostToolCall` audit event is
  persisted after execution.
- In-memory and PostgreSQL-backed tests prove tenant isolation for those
  audit queries.
- Behavioural tests demonstrate the workflow from the caller's perspective.

Quality criteria:

- Tests: new `rstest`, in-memory integration, PostgreSQL integration, and
  `rstest-bdd` scenarios pass.
- Lint and formatting: `make fmt`, `make check-fmt`, and `make lint` pass.
- Documentation: `make markdownlint` and `make nixie` pass after updating
  the design doc, user guide, and roadmap.

## Idempotence and recovery

- Re-running the migration in a fresh test database is safe; the template
  database version must be bumped whenever the migration list changes.
- Re-running focused tests is the first retry path after any failure.
- If `make fmt` rewrites unrelated Markdown files, restore unrelated churn
  before finalising.
- If PostgreSQL tests appear to use an old schema, verify that
  `tests/postgres/helpers.rs:TEMPLATE_DB` was bumped and the migration list was
  updated in order.

## Outcomes & retrospective

Implemented outcome:

- `hook_engine` owns a queryable policy-audit model and persistence port.
- `tool_registry` enforces `PreToolCall` policy and records `PostToolCall`
  audit through a local governance port.
- Unit, in-memory integration, PostgreSQL integration, and behavioural
  tests prove happy paths, unhappy paths, and key edge cases.
- `docs/corbusier-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`
  reflect the implemented behaviour.

Retrospective:

- Dedicated execution-scope objects kept workflow correlation out of
  `RequestContext` while still making policy audit rows queryable by task and
  conversation.
- A separate policy-audit projection table was the right trade-off; indexed
  rows are materially simpler to query than hook execution JSON.
- Environment-level PostgreSQL sharp edges were infrastructure problems
  rather than application-code problems.

## Revision note

Revision 0 (2026-03-21): initial draft created.

Revision 1 (2026-03-27): reduced the primary ExecPlan below the repository line
limit, moved interface details and command checklists into companion documents,
and marked Stage F complete to match the finished validation work.
