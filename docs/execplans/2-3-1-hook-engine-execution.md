# Implement hook engine execution (Roadmap 2.3.1)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

This document plans roadmap item 2.3.1 in `docs/roadmap.md`:

- Define hook triggers for commit, merge, and deploy events.
- Execute hooks with structured outcomes and logs.
- Record execution results for every configured trigger.

Execution phase has not started. Implementation begins only after explicit
approval of this plan.

## Purpose / big picture

After this change, Corbusier will run governance hooks whenever commit, merge,
or deploy events occur, and will persist structured execution results for audit
and policy follow-up. This delivers roadmap 2.3.1 and creates the execution
substrate required for roadmap 2.3.2 policy enforcement.

Observable operator outcome:

1. A configured trigger event causes matching hooks to run in deterministic
   order.
2. Every configured hook run emits a structured result with status, action
   outcomes, and execution log entries.
3. Hook execution results are queryable from storage for each configured
   trigger type.

## Constraints

- Preserve hexagonal boundaries:
  - Domain and service logic must remain infrastructure-free.
  - Ports define contracts owned by the hook engine module.
  - Adapters implement ports (memory and postgres), with no adapter-to-adapter
    calls.
- Respect roadmap dependency ordering:
  - 1.2.3 (task transition enforcement) and 2.1.2 (tool discovery and routing)
    must be complete before wiring trigger producers.
- Implement trigger coverage for exactly these roadmap-required event families
  in this milestone: commit, merge, deploy.
- Model execution output per `docs/corbusier-design.md` section 6.3.3
  (`HookExecutionResult` style structured outcomes and logs).
- Use `rstest` for unit/integration test fixtures and parameterized cases.
- Use `rstest-bdd` for behavioural scenarios where user-observable workflow
  behaviour is exercised.
- Use `pg-embedded-setup-unpriv` fixtures for postgres-backed tests.
- Keep files below 400 lines by splitting domain, port, adapter, and test code
  into focused modules.
- Documentation deliverables are mandatory before roadmap completion:
  - update `docs/corbusier-design.md` with any design decisions taken;
  - update `docs/users-guide.md` with new user-visible behaviour;
  - update `docs/roadmap.md` and mark 2.3.1 done only after all gates pass.

## Tolerances (exception triggers)

- Scope tolerance: stop and escalate if implementation exceeds 30 files changed
  or 2,000 net new lines.
- Interface tolerance: stop and escalate if existing public APIs from completed
  modules must change incompatibly.
- Dependency tolerance: stop and escalate if a new external crate is required.
- Data tolerance: stop and escalate if schema changes beyond adding
  `hook_executions` and supporting indexes are required.
- Iteration tolerance: stop and escalate if the same failure persists after
  three focused fix attempts.
- Ambiguity tolerance: stop and escalate if merge/deploy event ownership
  between tool-routing and VCS integration remains ambiguous after reviewing
  section 2.1.3 and section 6.3.3 of the design doc.

## Risks

- Risk: Trigger source ambiguity for merge/deploy before full VCS integration.
  Severity: medium Likelihood: medium Mitigation: introduce explicit
  trigger-event input port for hook engine and keep upstream producers as
  adapters; avoid coupling to unfinished VCS module.

- Risk: Hook result schema may be underspecified for future policy engine.
  Severity: medium Likelihood: medium Mitigation: store action outcomes and
  logs as structured JSON values with stable keys, plus typed domain wrappers
  to preserve compatibility.

- Risk: Behavioural tests become brittle due to step-definition mismatch.
  Severity: low Likelihood: medium Mitigation: keep `.feature` wording simple,
  reuse existing `rstest-bdd` fixture conventions, and keep scenario file names
  distinct from step module directory names.

- Risk: Postgres integration flakiness from embedded cluster lifecycle.
  Severity: low Likelihood: medium Mitigation: reuse existing shared
  cluster/template-db helpers and follow existing retry discipline in
  validation.

## Progress

- [x] (2026-02-28 13:05Z) Gathered roadmap/design/testing constraints and
      drafted this ExecPlan.
- [ ] Stage A: Finalize hook execution domain model and port contracts.
- [ ] Stage B: Implement hook engine service orchestration and trigger mapping.
- [ ] Stage C: Implement in-memory and postgres adapters (including migration).
- [ ] Stage D: Add unit, integration, and behavioural tests.
- [ ] Stage E: Update design and user docs; mark roadmap 2.3.1 done.
- [ ] Stage F: Run full quality gates and capture validation evidence.

## Surprises & discoveries

- The design schema in section 6.3.3 lists `PreCommit`, `PostCommit`, and pull
  request triggers, while roadmap 2.3.1 explicitly asks for commit, merge, and
  deploy trigger families. This plan resolves the mismatch by introducing a
  domain trigger enum centred on roadmap semantics while allowing adapter-level
  mapping from concrete event sources.
- Current source tree has no hook-engine module yet; this work introduces a new
  top-level subsystem with the same hexagonal shape as `task` and
  `agent_backend`.

## Decision log

- Decision: implement hook execution as a new `hook_engine` subsystem with
  domain/ports/adapters/services/test modules. Rationale: keeps 2.3.x
  governance work cohesive and avoids leaking hook logic into message/task/tool
  adapters. Date/Author: 2026-02-28 / plan author.

- Decision: treat commit/merge/deploy as canonical domain trigger types,
  independent of upstream event producers. Rationale: aligns directly with
  roadmap success criteria and allows adapter mapping from tool-router/VCS
  events without reworking core logic. Date/Author: 2026-02-28 / plan author.

## Outcomes & retrospective

Implementation not started. This section will be updated during execution with:

- what shipped;
- whether acceptance criteria were met;
- any residual gaps, risks, or follow-up work.

## Context and orientation

Repository baseline relevant to this milestone:

- Existing completed domains are in `src/message/`, `src/task/`, and
  `src/agent_backend/` with consistent hexagonal layout.
- Tool plane and workflow governance modules for roadmap 2.x do not yet exist.
- Postgres test migration wiring is centralized in
  `tests/postgres/helpers.rs` through constants and `apply_migrations()`.
- Integration test entry points are `tests/in_memory.rs` and
  `tests/postgres.rs`.
- Behavioural test scenarios live in `tests/features/` and are wired via
  `tests/*_steps.rs` or `tests/*_scenarios.rs` with `rstest-bdd` macros.

Planned subsystem layout for this feature:

```plaintext
src/hook_engine/
├── mod.rs
├── domain/
│   ├── mod.rs
│   ├── action.rs
│   ├── definition.rs
│   ├── error.rs
│   ├── execution.rs
│   ├── ids.rs
│   └── trigger.rs
├── ports/
│   ├── mod.rs
│   ├── definition_repository.rs
│   ├── execution_log.rs
│   └── trigger_event_source.rs
├── adapters/
│   ├── mod.rs
│   ├── memory/
│   │   ├── mod.rs
│   │   ├── definition_repository.rs
│   │   └── execution_log.rs
│   └── postgres/
│       ├── mod.rs
│       ├── models.rs
│       ├── repository.rs
│       └── schema.rs
├── services/
│   ├── mod.rs
│   └── engine.rs
└── tests/
    ├── mod.rs
    ├── domain_tests.rs
    └── service_tests.rs
```

## Plan of work

### Stage A: Domain and contract scaffolding

Create core domain types and contracts without wiring to infrastructure:

- Define `HookTriggerType` with `Commit`, `Merge`, and `Deploy`.
- Define `HookTriggerContext` carrying trigger metadata required for audit
  correlation (task/conversation refs, source event ID, timestamp, optional
  actor).
- Define `HookDefinition` and `HookAction` as declarative inputs.
- Define `HookExecutionResult` and `ActionResult` with typed status enum,
  structured action outputs, and log entries.
- Define typed domain errors for validation and execution failures.
- Define port traits:
  - definition lookup by trigger type;
  - execution result persistence and lookup;
  - optional trigger event feed abstraction for adapter-driven execution.

Go/no-go: proceed only when domain and ports compile and unit tests validate
trigger/action/result invariants.

### Stage B: Hook engine service orchestration

Implement `HookEngineService` in `src/hook_engine/services/engine.rs`:

- Input: `HookTriggerContext`.
- Resolve enabled hook definitions for the trigger.
- Execute hook actions in deterministic order (`priority`, then `hook_id`).
- Collect per-action results and normalized log entries.
- Compute overall hook status from action statuses.
- Persist each hook execution result via execution-log port.
- Return structured result list to caller.

Go/no-go: proceed only when service-level unit tests pass for success, partial
failure, total failure, and zero-hook cases.

### Stage C: Adapters and persistence

Implement adapters:

- In-memory definition repository and execution-log repository for fast unit and
  integration tests.
- Postgres execution-log repository backed by a new migration adding
  `hook_executions` table (matching design section 6.2.1 storage expectations).
- Update postgres test migration plumbing in `tests/postgres/helpers.rs`.
- Register module exports in `src/lib.rs` and `src/hook_engine/mod.rs`.

Expected migration shape:

- `id` UUID primary key.
- `trigger_context_id` UUID.
- `hook_id` text.
- `trigger_type` text.
- `predicate_data` JSONB.
- `action_results` JSONB.
- `status` text.
- `executed_at` timestamptz.
- supporting indexes on `(trigger_type, executed_at)` and
  `(trigger_context_id)`.

Go/no-go: proceed only when postgres adapter tests pass against embedded
postgres.

### Stage D: Trigger producer wiring (commit, merge, deploy)

Integrate with the post-2.1.2 execution path:

- Add adapter mapping from upstream workflow events to domain triggers:
  - commit event -> `HookTriggerType::Commit`;
  - merge event -> `HookTriggerType::Merge`;
  - deploy event -> `HookTriggerType::Deploy`.
- Ensure every configured trigger invocation calls hook engine once per event.
- Ensure returned execution results are attached to audit/event pipeline for
  downstream policy and reporting.

Go/no-go: proceed only when integration tests prove trigger mapping and result
persistence for all three trigger types.

### Stage E: Test coverage

Add tests across layers.

Unit tests with `rstest`:

- domain validation for definitions, triggers, and statuses;
- deterministic ordering by priority and hook ID;
- status aggregation rules (all success, mixed, all fail);
- structured log shape and required fields.

Integration tests (in-memory and postgres):

- commit trigger executes configured hooks and persists results;
- merge trigger with failing action persists failure status and logs;
- deploy trigger with no hooks records no execution rows;
- repeated trigger invocations create separate execution rows.

Behavioural tests with `rstest-bdd`:

- feature scenarios in `tests/features/hook_engine_execution.feature` covering
  happy path and unhappy path;
- step definitions in `tests/hook_engine_execution_steps/`;
- scenario entrypoint file distinct from step directory name.

### Stage F: Documentation and roadmap updates

Update docs after implementation behaviour is stable:

- `docs/corbusier-design.md`: add implementation decisions for trigger mapping,
  status model, and persistence schema refinements.
- `docs/users-guide.md`: add a user-facing section explaining configured
  triggers, result records, and failure semantics.
- `docs/roadmap.md`: mark 2.3.1 and sub-items as done only after all quality
  gates pass.

## Concrete steps

Run all commands from `/home/user/project`.

1. Baseline and red-green setup:

```bash
set -o pipefail
cargo test --all-targets --all-features hook_engine 2>&1 | tee /tmp/2-3-1-pre.log
```

Expected: failures due to missing hook engine module/tests before
implementation.

1. Implement stages A-D, then run formatting and lint/test gates:

```bash
set -o pipefail
make check-fmt 2>&1 | tee /tmp/2-3-1-check-fmt.log
set -o pipefail
make lint 2>&1 | tee /tmp/2-3-1-lint.log
set -o pipefail
make test 2>&1 | tee /tmp/2-3-1-test.log
```

Expected: all commands exit 0.

1. Validate documentation updates:

```bash
set -o pipefail
make fmt 2>&1 | tee /tmp/2-3-1-fmt.log
set -o pipefail
PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/2-3-1-markdownlint.log
set -o pipefail
make nixie 2>&1 | tee /tmp/2-3-1-nixie.log
```

Expected: all commands exit 0; only intended files remain modified.

## Validation and acceptance

Acceptance criteria for roadmap 2.3.1 are met when all of the following are
true:

- Trigger coverage:
  - commit, merge, and deploy events each map to hook execution.
- Structured outcome coverage:
  - each executed hook produces a stored result with hook ID, trigger type,
    status, action results, and logs.
- Persistence coverage:
  - querying persistence returns execution records for every configured trigger
    invocation in tests.
- Test coverage:
  - new `rstest` unit tests pass for happy/unhappy paths and edge cases;
  - new integration tests pass for memory and postgres adapters;
  - new `rstest-bdd` scenarios pass for workflow-level behaviour.
- Quality gates:
  - `make check-fmt`, `make lint`, `make test`, `make markdownlint`, and
    `make nixie` all pass.

## Idempotence and recovery

- All implementation steps are additive and re-runnable.
- Migration retries are safe if guarded by transactional DDL and idempotent test
  template setup.
- If `make fmt` rewrites unrelated docs, restore non-target files before
  finalizing and verify with `git status --short`.
- If postgres tests fail with transient embedded-cluster shutdown errors, rerun
  `make test` once and capture both logs before escalating.

## Artifacts and notes

Evidence to capture during implementation:

- log files from each gate in `/tmp/2-3-1-*.log`;
- names of new tests proving each trigger family;
- SQL migration identifier and schema diff summary;
- final `git status --short` showing only intended files;
- roadmap diff confirming 2.3.1 marked done.

## Interfaces and dependencies

Target interfaces (names may be refined during implementation, but intent must
remain):

```rust
pub enum HookTriggerType {
    Commit,
    Merge,
    Deploy,
}

pub trait HookDefinitionRepository: Send + Sync {
    async fn list_enabled_for_trigger(
        &self,
        trigger: HookTriggerType,
    ) -> HookResult<Vec<HookDefinition>>;
}

pub trait HookExecutionLogRepository: Send + Sync {
    async fn store(&self, result: &HookExecutionResult) -> HookResult<()>;
    async fn find_by_trigger_context(
        &self,
        trigger_context_id: TriggerContextId,
    ) -> HookResult<Vec<HookExecutionResult>>;
}

pub trait HookEngine {
    async fn execute(
        &self,
        context: HookTriggerContext,
    ) -> HookResult<Vec<HookExecutionResult>>;
}
```

Dependencies:

- Reuse existing workspace dependencies (`tokio`, `serde`, `diesel`, `rstest`,
  `rstest-bdd`, `pg-embed-setup-unpriv`, `mockable`) and avoid adding new
  crates unless escalation is approved.

## Revision note

- 2026-02-28: Initial draft created for roadmap 2.3.1 planning. No execution
  work performed yet.
