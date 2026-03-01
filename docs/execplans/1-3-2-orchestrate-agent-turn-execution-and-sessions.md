# Orchestrate agent turn execution and sessions (roadmap 1.3.2)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

This plan covers roadmap item 1.3.2 in [docs/roadmap.md](docs/roadmap.md):

- Coordinate turn execution with tool calls and responses.
- Maintain session state and expiry rules.
- Deliver consistent tool routing and session continuity.

Execution phase does not begin until this draft is explicitly approved.

## Purpose / big picture

After this change, Corbusier can run a complete agent turn through a backend
adapter, route tool calls through one orchestration path, and keep session
continuity across turns with explicit expiry handling. This removes ad-hoc
backend-specific sequencing and makes turn behaviour observable and testable.

Observable outcomes:

1. A turn request can be executed against a registered backend and yields a
   canonical result with tool call auditability.
2. Repeated turns in the same conversation reuse the active session until
   expiry, then rotate to a new session deterministically.
3. Tool calls emitted by backends are routed through one consistent router path
   and have deterministic ordering in audit records.

## Constraints

- Preserve hexagonal boundaries:
  - Domain logic in `src/agent_backend/domain/` must not import infrastructure
    crates or adapter modules.
  - Ports in `src/agent_backend/ports/` define contracts only.
  - Adapters in `src/agent_backend/adapters/` implement ports and contain
    infrastructure concerns.
- Do not regress roadmap 1.3.1 behaviour in backend registration and discovery.
- Keep canonical message semantics compatible with roadmap 1.1.1 types and
  validation guarantees.
- Use `rstest` for unit and integration tests, and `rstest-bdd` for behavioural
  scenarios where end-user behaviour is expressed.
- Use `pg-embedded-setup-unpriv` fixtures for Postgres integration coverage.
- Keep every Rust source file under 400 lines by extracting helper modules.
- Maintain strict lints (`-D warnings` via `make lint`) and avoid suppression
  unless unavoidable and narrowly scoped with reason.
- Update `docs/corbusier-design.md` with roadmap 1.3.2 implementation
  decisions.
- Update `docs/users-guide.md` with user-visible turn/session behaviour.
- Mark roadmap entry 1.3.2 complete in `docs/roadmap.md` only after all
  validation gates pass.

## Tolerances (exception triggers)

- Scope: stop and escalate if implementation exceeds 30 files changed or 1,800
  net lines.
- Interface: stop and escalate if existing public APIs for `message` or `task`
  require incompatible signature changes.
- Dependencies: stop and escalate if any new external crate is required.
- Data model: stop and escalate if session expiry cannot be represented without
  destructive schema changes.
- Iterations: stop and escalate if one failure class persists after 4 focused
  fix-and-rerun cycles.
- Ambiguity: stop and escalate if session expiry semantics cannot be made
  consistent with both `docs/corbusier-design.md` and existing handoff/session
  behaviour.

## Risks

- Risk: Session orchestration semantics overlap existing
  `message::domain::AgentSession` handoff sessions, creating duplication or
  conflicting state transitions. Severity: high Likelihood: medium Mitigation:
  define one session authority for 1.3.2 and enforce it through a dedicated
  port; add regression tests for handoff scenarios to ensure no behavioural
  drift.

- Risk: Tool routing contracts are underspecified before roadmap 2.1.1 (MCP
  lifecycle) lands. Severity: medium Likelihood: high Mitigation: define a
  narrow `ToolRouterPort` for deterministic routing and stubbed execution now,
  with schema translation deferred to roadmap 1.3.3.

- Risk: Expiry logic can become time-flaky if wall-clock access is not
  injected. Severity: medium Likelihood: medium Mitigation: use
  `mockable::Clock` in orchestration services and verify boundaries with
  deterministic clock-controlled tests.

- Risk: BDD steps for quoted tool payloads are brittle.
  Severity: low Likelihood: medium Mitigation: use simple, unambiguous step
  text patterns and scenario files that avoid escaped quote complexity.

## Progress

- [x] (2026-02-28 13:09Z) Reviewed roadmap/design/testing docs and existing
      1.3.1 implementation to draft this ExecPlan.
- [ ] Stage A: Finalize 1.3.2 domain model and port contracts.
- [ ] Stage B: Add failing unit and behavioural tests (red phase).
- [ ] Stage C: Implement orchestration service and in-memory adapters.
- [ ] Stage D: Implement Postgres adapter updates and migration updates if
      needed.
- [ ] Stage E: Add Postgres integration coverage with
      `pg-embedded-setup-unpriv` fixtures.
- [ ] Stage F: Update user/design docs and mark roadmap item done.
- [ ] Stage G: Run full quality gates and capture evidence logs.

## Surprises & discoveries

- Existing code already contains `message` session persistence and handoff
  workflows (`agent_sessions`, `handoffs`, `context_snapshots`). 1.3.2 must
  avoid creating a second competing session lifecycle.
- `docs/corbusier-design.md` defines the `AgentBackend` runtime trait and turn
  flow at conceptual level, but concrete Rust module/file mapping is not yet
  implemented; this plan provides that mapping.

## Decision log

- Decision: Implement 1.3.2 in the `agent_backend` module as the orchestration
  home, while integrating with canonical message/session types through explicit
  ports rather than direct adapter coupling. Rationale: keeps roadmap ownership
  aligned with section 1.3 while preserving hexagonal boundaries and avoiding
  service-level imports of adapter internals. Date/Author: 2026-02-28 / plan
  author.

- Decision: Treat session expiry as a first-class domain policy evaluated by
  service logic using injected clock. Rationale: ensures deterministic tests
  and explicit behaviour for rotation, continuation, and cleanup paths.
  Date/Author: 2026-02-28 / plan author.

## Outcomes & retrospective

Pending implementation.

Completion checklist for this section:

- Summarize delivered behaviour against each 1.3.2 success criterion.
- Record final test counts for unit, behavioural, in-memory integration, and
  Postgres integration suites.
- Capture any deferred work and why it was deferred.

## Context and orientation

Relevant repository state before 1.3.2 implementation:

- `src/agent_backend/` currently provides registration/discovery only
  (roadmap 1.3.1), with domain/ports/adapters/service built around
  `BackendRegistryService`.
- `src/message/domain/agent_session.rs` and corresponding ports/adapters already
  model persisted session lifecycle for handoff workflows.
- `docs/corbusier-design.md` section 2.2.3 defines F-003-RQ-002 and
  F-003-RQ-004 requirements; section 4.2.1.1 defines turn execution process.
- Integration test entrypoints are `tests/in_memory.rs` and `tests/postgres.rs`.
- Behaviour scenarios are under `tests/features/` and are wired through
  dedicated scenario entrypoint files in `tests/`.

Primary files expected to change:

- `src/agent_backend/domain/mod.rs` plus new domain modules for turn/session
  orchestration.
- `src/agent_backend/ports/mod.rs` plus new ports for runtime backend
  execution, tool routing, and session persistence policy access.
- `src/agent_backend/services/mod.rs` plus new orchestration service module.
- `src/agent_backend/adapters/memory/` and
  `src/agent_backend/adapters/postgres/` for runtime/session adapter support.
- `src/agent_backend/tests/` unit tests for orchestration.
- `tests/in_memory/` and `tests/postgres/` integration suites.
- `tests/features/` and new step definition modules for behavioural coverage.
- `docs/corbusier-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`.

## Plan of work

### Stage A: Define orchestration model and contracts (design-first)

Implement domain and port contracts for turn orchestration without wiring full
behaviour yet.

Planned edits:

- Add `src/agent_backend/domain/turn.rs` with canonical request/result/event
  types used by orchestration.
- Add `src/agent_backend/domain/session.rs` with session lifecycle policy types
  (active, expired, rotated) and expiry evaluation helpers.
- Add `src/agent_backend/domain/error.rs` variants for orchestration failures
  (backend unavailable, tool routing failure, session expired/not found).
- Add `src/agent_backend/ports/runtime.rs` defining backend runtime execution
  port (create/reuse session + execute turn).
- Add `src/agent_backend/ports/tool_router.rs` defining deterministic tool call
  routing contract.
- Add `src/agent_backend/ports/session.rs` defining session lookup/update and
  expiry-aware persistence contract.
- Update `mod.rs` re-exports for new domain and port surfaces.

Go/no-go validation for Stage A:

- `cargo test --lib agent_backend::tests::domain_tests` compiles and runs.
- No adapter imports inside new domain files.

### Stage B: Add failing tests first (red phase)

Write tests that encode required behaviour before service implementation.

Planned tests:

- Unit tests in `src/agent_backend/tests/turn_orchestration_tests.rs` using
  `rstest`:
  - happy path: one turn with one tool call routes correctly;
  - unhappy path: backend execution error;
  - unhappy path: tool routing failure;
  - edge: session at expiry boundary rotates on next turn;
  - edge: deterministic tool routing order across repeated identical input.
- Behaviour tests with `rstest-bdd`:
  - `tests/features/agent_turn_orchestration.feature`;
  - steps in `tests/agent_turn_orchestration_steps/`;
  - scenario entrypoint `tests/agent_turn_orchestration_scenarios.rs`.
- In-memory integration tests in
  `tests/in_memory/agent_turn_orchestration_tests.rs`.
- Register new test modules in `tests/in_memory.rs` and `tests/postgres.rs`
  (Postgres module stub can be failing/ignored until Stage E wiring).

Go/no-go validation for Stage B:

- New tests fail for expected missing-orchestration reasons.
- Existing 1.3.1 tests remain passing.

### Stage C: Implement orchestration service and memory adapters (green phase)

Implement service logic to satisfy Stage B tests with deterministic behaviour.

Planned edits:

- Add `src/agent_backend/services/orchestrator.rs` with
  `AgentTurnOrchestratorService`.
- Service flow follows design section 4.2.1.1:
  1. Build turn context and validate backend registration/status.
  2. Resolve or create active session.
  3. Apply expiry policy and rotate session if required.
  4. Execute backend turn request.
  5. Route tool calls via `ToolRouterPort` in deterministic order.
  6. Aggregate response + tool audits.
  7. Persist session updates and turn execution record.
- Add in-memory runtime and session adapters under
  `src/agent_backend/adapters/memory/` for deterministic tests.
- Keep orchestration logic in service/domain; adapters should only map storage
  and transport details.

Go/no-go validation for Stage C:

- New unit + in-memory integration tests pass.
- Determinism tests pass on repeated execution.

### Stage D: Postgres adapter and persistence updates

Persist orchestration session continuity and expiry state in Postgres.

Planned edits:

- Add/extend Postgres adapter modules under
  `src/agent_backend/adapters/postgres/`
  for orchestration session persistence and turn execution metadata.
- If expiry fields are missing, add one additive migration under `migrations/`
  and wire it into `tests/postgres/helpers.rs` template migration list.
- Update Diesel schema/model files corresponding to new/changed tables.

Go/no-go validation for Stage D:

- Postgres adapter compiles cleanly under `make lint`.
- Repository-level integration tests for store/load/expiry queries pass.

### Stage E: Behavioural and Postgres integration hardening

Complete end-to-end behavioural checks and Postgres-backed orchestration checks.

Planned tests:

- `tests/postgres/agent_turn_orchestration_tests.rs` using
  `pg_embedded_setup_unpriv::test_support::shared_test_cluster` fixture
  patterns already used in the repository.
- Ensure both happy and unhappy paths are covered:
  - successful turn with routed tool calls;
  - session reuse before expiry;
  - session rotation after expiry;
  - backend failure and tool router failure propagation.
- `rstest-bdd` scenarios validate user-observable continuity semantics.

Go/no-go validation for Stage E:

- Postgres orchestration tests pass consistently.
- BDD scenarios pass with deterministic results.

### Stage F: Documentation and roadmap closure

Update user-facing and design documentation once behaviour is proven.

Planned edits:

- `docs/corbusier-design.md`: append
  `Implementation decisions (YYYY-MM-DD) — roadmap 1.3.2` beneath F-003-RQ-002
  and F-003-RQ-004, documenting chosen session and routing approach.
- `docs/users-guide.md`: add section describing turn orchestration behaviour,
  tool routing consistency, and session expiry/rotation semantics with example.
- `docs/roadmap.md`: mark 1.3.2 and child bullets as done only after full gate
  pass.

Go/no-go validation for Stage F:

- Documentation reflects implemented behaviour with no stale statements.

### Stage G: Full quality gates and evidence capture

Run all required gates and capture logs for auditability.

Commands from repository root:

```bash
set -o pipefail; make check-fmt 2>&1 | tee /tmp/1-3-2-check-fmt.log
set -o pipefail; make lint 2>&1 | tee /tmp/1-3-2-lint.log
set -o pipefail; make test 2>&1 | tee /tmp/1-3-2-test.log
set -o pipefail; make fmt 2>&1 | tee /tmp/1-3-2-fmt.log
set -o pipefail; PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/1-3-2-markdownlint.log
set -o pipefail; make nixie 2>&1 | tee /tmp/1-3-2-nixie.log
```

Acceptance for Stage G:

- All commands exit successfully.
- `git status --short` contains only intended files.
- Roadmap item 1.3.2 is checked off.

## Concrete steps

1. Create domain and port files for orchestration in `src/agent_backend/` and
   update module exports.
2. Add red-phase tests across unit/in-memory/BDD/Postgres skeletons.
3. Implement service orchestration and memory adapters to satisfy red tests.
4. Implement or extend Postgres persistence for session expiry continuity.
5. Complete Postgres integration and BDD behavioural assertions.
6. Update docs (`corbusier-design`, `users-guide`, roadmap).
7. Run full quality gates with `tee` logs and capture outcomes in this plan.

Expected short validation transcripts during implementation:

```plaintext
$ make lint
...
Finished ...
```

```plaintext
$ make test
...
Summary [ ... ]
```

## Validation and acceptance

Behavioural acceptance criteria:

- A registered active backend can execute a turn and return canonical turn
  output including tool audit data.
- Tool calls are routed via one orchestration path with deterministic ordering
  across identical runs.
- Session continuity is preserved across turns in one conversation until expiry.
- When session expiry threshold is crossed, next turn rotates to a new active
  session and continues execution.
- Failure paths return typed errors without corrupting session state.

Quality criteria:

- Tests: new `rstest` unit/integration tests and `rstest-bdd` scenarios pass.
- Lint/type/doc: `make lint` passes with no warnings.
- Formatting: `make check-fmt` and markdown checks pass.
- Runtime persistence: Postgres integration tests pass using
  `pg-embedded-setup-unpriv` fixtures.

## Idempotence and recovery

- All implementation steps are additive and re-runnable.
- If `make fmt` rewrites unrelated Markdown files, restore unintended files and
  rerun `make markdownlint`.
- If transient Postgres shutdown errors occur during `make test`, rerun once;
  persistent failures require investigation and should be logged in
  `Surprises & Discoveries`.
- Migration changes must be additive; do not alter historical migration files.

## Artifacts and notes

Store verification logs at:

- `/tmp/1-3-2-check-fmt.log`
- `/tmp/1-3-2-lint.log`
- `/tmp/1-3-2-test.log`
- `/tmp/1-3-2-fmt.log`
- `/tmp/1-3-2-markdownlint.log`
- `/tmp/1-3-2-nixie.log`

Add concise excerpts proving success/failure states as implementation proceeds.

## Interfaces and dependencies

Planned interface surface at completion (exact names may vary, behaviour must
match):

```rust
#[async_trait::async_trait]
pub trait AgentRuntimePort: Send + Sync {
    async fn create_or_resume_session(
        &self,
        backend_id: BackendId,
        conversation_id: ConversationId,
    ) -> Result<RuntimeSessionHandle, AgentRuntimeError>;

    async fn execute_turn(
        &self,
        session: &RuntimeSessionHandle,
        request: TurnExecutionRequest,
    ) -> Result<TurnExecutionStream, AgentRuntimeError>;
}

#[async_trait::async_trait]
pub trait ToolRouterPort: Send + Sync {
    async fn route_tool_call(
        &self,
        call: PlannedToolCall,
        context: ToolRoutingContext,
    ) -> Result<ToolRoutingResult, ToolRoutingError>;
}

#[async_trait::async_trait]
pub trait AgentTurnSessionRepository: Send + Sync {
    async fn find_active(
        &self,
        conversation_id: ConversationId,
    ) -> Result<Option<OrchestrationSession>, SessionStoreError>;

    async fn store_or_update(
        &self,
        session: &OrchestrationSession,
    ) -> Result<(), SessionStoreError>;

    async fn expire_before(
        &self,
        deadline: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, SessionStoreError>;
}
```

Dependencies remain within existing crate set (`mockable`, `rstest`,
`rstest-bdd`, Diesel, Tokio, `pg-embedded-setup-unpriv`) unless escalated.

## Revision note

- 2026-02-28: Initial draft created from roadmap 1.3.2 requirements and design
  references (`docs/corbusier-design.md` §2.2.3 and §4.2.1.1), with explicit
  stage gates, testing matrix, documentation obligations, and closure criteria.
