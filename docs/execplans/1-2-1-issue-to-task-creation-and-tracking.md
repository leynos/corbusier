# Implement Issue-to-Task Creation and Tracking (Roadmap 1.2.1)

This ExecPlan is a living document. The sections `Constraints`, `Tolerances`,
`Risks`, `Progress`, `Surprises & Discoveries`, `Decision Log`, and
`Outcomes & Retrospective` must be kept up to date as work proceeds.

Status: DRAFT

`PLANS.md` is not present in this repository as of 2026-02-09, so this plan is
the controlling execution document for this feature.

## Purpose / big picture

Implement roadmap item 1.2.1, so Corbusier can create an internal task record
from external issue metadata, assign internal task identity and lifecycle
timestamps, and retrieve the task by external issue reference.

After this change, an implementer can call task orchestration code with issue
input (GitHub/GitLab issue reference plus metadata), persist the task, and
perform reverse lookup by issue reference. Success is observable in unit tests,
behaviour tests, and PostgreSQL integration tests that prove both happy and
unhappy paths.

## Constraints

- Keep strict hexagonal boundaries:
  - Domain logic must not depend on Diesel, PostgreSQL, or transport details.
  - Ports are defined in the core module and implemented only by adapters.
  - Adapters do not call each other directly.
- Preserve existing message/handoff behaviour and public interfaces unless a
  change is explicitly required for task lifecycle support.
- Use typed domain errors (`thiserror`) for inspectable failures.
- Use `rstest` for unit/integration fixtures and `rstest-bdd` v0.5.0 for
  behavioural tests.
- Use `pg-embed-setup-unpriv` v0.5.0 for local PostgreSQL-backed tests.
- Maintain repository quality bars:
  - No clippy warnings.
  - Formatting and lint checks pass.
  - Full workspace tests pass.
- Each Rust module touched/added must include module-level `//!` docs.
- Keep file sizes manageable; split modules before crossing the 400-line rule.
- Update docs when behaviour changes:
  `docs/corbusier-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`.

## Tolerances (exception triggers)

- Scope: if implementation exceeds 30 files or 2,200 net lines, stop and
  escalate with a reduced-scope option.
- Interfaces: if implementing 1.2.1 requires changing unrelated public APIs
  outside the new task lifecycle boundary, stop and escalate.
- Dependencies: if any new crate beyond requested version bumps is needed, stop
  and escalate with rationale.
- Data model: if uniqueness cannot be enforced for issue-origin tasks without
  broad schema redesign, stop and escalate.
- Iterations: if the same failing test remains unresolved after 3 focused
  attempts, stop and record alternatives.
- Timebox: if any single stage exceeds 4 hours of active work, stop and report
  remaining unknowns.
- Ambiguity: if issue reference parsing rules are materially unclear (URL
  normalization, provider inference), stop and request a decision.

## Risks

- Risk: Existing migrations do not currently create a `tasks` table, while
  schema and design references for task lifecycle assume one exists. Severity:
  high
  Likelihood: high
  Mitigation: add a dedicated migration for task lifecycle foundations and
  verify Diesel schema regeneration plus migration ordering.

- Risk: Duplicate issue-task creation if uniqueness is checked only in-memory.
  Severity: high Likelihood: medium Mitigation: enforce uniqueness in adapter
  logic and at the database level with an index/constraint over canonical issue
  reference fields.

- Risk: Behaviour tests become brittle if they assert internal representation
  instead of observable behaviour. Severity: medium Likelihood: medium
  Mitigation: keep `Then` assertions focused on task creation/retrieval
  outcomes and error surfaces.

- Risk: Version upgrade friction when moving `rstest-bdd` and
  `pg-embed-setup-unpriv` to v0.5.0. Severity: medium. Likelihood: medium.
  Mitigation: update dependency versions first and confirm compile/test
  baseline before feature logic.

## Progress

- [x] (2026-02-09 02:49Z) Gathered roadmap and design requirements from
  `docs/roadmap.md` and `docs/corbusier-design.md` (§2.2.2, §4.3.1.2, §6.2.1).
- [x] (2026-02-09 02:49Z) Mapped existing architecture and test harness
  patterns in `src/message/` and `tests/postgres/`.
- [x] (2026-02-09 02:49Z) Authored initial ExecPlan draft for roadmap 1.2.1.
- [ ] Await user approval of this ExecPlan before implementation.
- [ ] Implement stages A-D and keep this section updated per stopping point.

## Surprises & Discoveries

- Observation: Model Context Protocol (MCP) resources for the Qdrant memory
  protocol were unavailable in this session (`list_mcp_resources` returned
  empty results). Evidence: tool discovery returned no configured
  resources/templates. Impact: implementation proceeds without historical
  project notes; capture new discoveries explicitly in repository docs.

- Observation: current schema tracks `conversations.task_id`, but migrations do
  not define a `tasks` table yet. Evidence:
  `migrations/2026-01-15-000000_create_base_tables/up.sql`. Impact: 1.2.1 must
  include persistent task table introduction before adapter integration can be
  complete.

## Decision Log

- Decision: implement task lifecycle in a dedicated `task` bounded context
  (`src/task/`) instead of extending `src/message/`. Rationale: task lifecycle
  is a separate aggregate with separate ports and adapters; this keeps domain
  boundaries explicit and avoids cross-feature coupling. Date/Author:
  2026-02-09 / Codex.

- Decision: store canonical issue origin in domain value objects and persist as
  JSONB `origin`, with typed extraction helpers in adapters. Rationale: aligns
  with `docs/corbusier-design.md` task-origin schema while keeping provider
  metadata extensible. Date/Author: 2026-02-09 / Codex.

- Decision: include roadmap completion (`[x]`) only at end of implementation,
  not during planning. Rationale: roadmap should represent delivered behaviour,
  not planned work. Date/Author: 2026-02-09 / Codex.

## Outcomes & Retrospective

Initial planning outcome: requirements and architecture constraints are now
captured in an executable implementation sequence. Outcomes, deviations,
and lessons learned will be filled in after execution.

## Context and orientation

Current repository state:

- Core crate exports `message` and `worker` modules in `src/lib.rs`.
- Message subsystem already follows hexagonal style:
  - Domain: `src/message/domain/`
  - Ports: `src/message/ports/`
  - Adapters: `src/message/adapters/`
  - Services: `src/message/services/`
- PostgreSQL integration harness already exists and should be reused:
  - Cluster/fixtures: `tests/postgres/cluster/`, `tests/postgres/helpers.rs`
  - Embedded PG bootstrap: `pg-embed-setup-unpriv`
- Behaviour tests already use `rstest-bdd` macros and feature files:
  - `tests/features/agent_handoff.feature`
  - `tests/agent_handoff_steps/`
- `docs/roadmap.md` item 1.2.1 is currently unchecked and depends on design
  guidance in `docs/corbusier-design.md` §2.2.2 and §4.3.1.2.

Target architecture for 1.2.1:

- Add a new `task` module mirroring the established layout:
  - `src/task/domain/` for `Task`, `TaskId`, `TaskOrigin`, `IssueRef`,
    timestamps, and validation.
  - `src/task/ports/` for repository and issue-origin lookup contracts.
  - `src/task/adapters/memory/` and `src/task/adapters/postgres/`.
  - `src/task/services/` for orchestration use cases
    (`create_from_issue`, `find_by_issue_ref`).
- Wire module exports in `src/lib.rs` and `src/task/mod.rs`.
- Keep message and task contexts decoupled; integration points are typed IDs and
  persisted relational references only.

## Plan of work

### Stage A: design lock and scaffolding contract (no behaviour changes)

Define task domain contracts and storage requirements before implementation:

- Specify domain types and invariants in `src/task/domain/`:
  - canonical `IssueRef` (provider, repository, issue number),
  - issue metadata mapping object,
  - task identity and lifecycle timestamps,
  - initial state (`draft`) for newly created issue-origin tasks.
- Define ports in `src/task/ports/`:
  - create task from issue origin metadata,
  - retrieve task by canonical issue reference,
  - detect duplicate issue-task associations.
- Add/adjust migration(s) for `tasks` persistence and retrieval indexes.
- Update Diesel schema/model scaffolding for the new table.

Go/no-go: do not proceed until the domain contracts are compile-clean and match
`docs/corbusier-design.md` requirements.

### Stage B: tests first (failing by design)

Add tests that describe required behaviour before final adapter/service logic:

- Unit tests (`rstest`) for domain mapping and validation:
  - valid metadata mapping,
  - invalid references (bad provider/repo/issue number),
  - timestamp generation semantics.
- Adapter/service tests for unhappy paths:
  - duplicate issue origin rejected,
  - missing task on lookup returns `None`/typed not-found result.
- Behaviour tests (`rstest-bdd` v0.5.0) in a new feature file, for example
  `tests/features/task_issue_creation.feature`, with scenarios covering:
  - successful creation from issue,
  - retrieval by external reference,
  - duplicate creation failure.
- PostgreSQL integration tests using `pg-embed-setup-unpriv` fixtures to prove
  persistence and lookup behaviour against real SQL.

Go/no-go: proceed only once new tests fail for the expected, feature-missing
reasons.

### Stage C: implementation (minimal code to satisfy tests)

Implement domain, service, and adapter logic in small increments:

- Implement domain constructors/validators and typed errors.
- Implement in-memory adapter for fast tests and behavioural wiring.
- Implement PostgreSQL adapter with Diesel row conversion helpers.
- Enforce issue uniqueness at repository level and DB index level.
- Generate task identifiers and lifecycle timestamps during creation via
  injected clock/ID provider patterns where needed.
- Ensure retrieval by canonical issue reference is supported in both in-memory
  and PostgreSQL adapters.

Go/no-go: proceed only when unit, behavioural, and integration tests for 1.2.1
all pass locally.

### Stage D: hardening, docs, and roadmap closure

- Refactor hotspots if complexity rises (split large functions/modules).
- Update design decisions in `docs/corbusier-design.md` with chosen mapping,
  uniqueness strategy, and timestamp lifecycle rationale.
- Update `docs/users-guide.md` with user-visible task creation/retrieval
  behaviour and examples.
- Mark roadmap item 1.2.1 and its sub-bullets as done in `docs/roadmap.md`
  only after all quality gates pass.
- Run full repository gates and documentation validators.

Go/no-go: complete only when all quality gates are green and docs match shipped
behaviour.

## Concrete steps

All commands run from repository root: `/home/user/project`.

1. Establish dependency and compile baseline.

   `cargo update -p rstest-bdd -p rstest-bdd-macros -p pg-embed-setup-unpriv`

   Expected signal: lockfile updates cleanly and subsequent build resolves
   v0.5.0 dependencies.

2. Add task module scaffolding and migration/schema updates.

   `make check-fmt`

   Expected signal: formatter check passes after scaffolding edits.

3. Run targeted tests while developing.

   `cargo nextest run --all-targets --all-features task`

   Expected signal: new task-focused suites fail first, then pass after
   implementation.

4. Run behaviour tests explicitly for new feature coverage.

   `cargo test --test task_issue_creation_steps`

   Expected signal: `Given/When/Then` scenarios pass with observable outcomes.

5. Run PostgreSQL task integration tests.

   `cargo test --test postgres -- task_lifecycle`

   Expected signal: embedded PostgreSQL tests pass for create and lookup flows.

6. Run commit gates with durable logs.

   `set -o pipefail; make check-fmt 2>&1 | tee /tmp/1-2-1-check-fmt.log`

   `set -o pipefail; make lint 2>&1 | tee /tmp/1-2-1-lint.log`

   `set -o pipefail; make test 2>&1 | tee /tmp/1-2-1-test.log`

   `set -o pipefail; make markdownlint 2>&1 | tee /tmp/1-2-1-markdownlint.log`

   `set -o pipefail; make nixie 2>&1 | tee /tmp/1-2-1-nixie.log`

   Expected signal: all commands exit 0 and logs contain no denied warnings.

## Validation and acceptance

Behavioural acceptance criteria:

- Creating a task from valid external issue metadata succeeds and returns a
  task that includes all the following:
  - generated internal task ID,
  - initial lifecycle state,
  - populated created/updated timestamps.
- Retrieving by the same external issue reference returns the created task.
- Creating a second task for the same issue reference fails with a typed,
  auditable duplicate-origin error.

Test acceptance criteria:

- Unit tests (`rstest`) cover:
  - valid/invalid issue metadata mapping,
  - timestamp generation semantics,
  - domain validation errors.
- Behaviour tests (`rstest-bdd` v0.5.0) cover at least:
  - happy path create + retrieve,
  - duplicate issue failure path,
  - lookup miss path.
- PostgreSQL integration tests cover:
  - persisted create + retrieve by issue reference,
  - uniqueness enforcement at DB level.

Quality criteria:

- `make check-fmt` passes.
- `make lint` passes with no warnings.
- `make test` passes across workspace.
- Documentation checks pass via `make markdownlint` and `make nixie`.

## Idempotence and recovery

- Development/test runs are repeatable:
  - in-memory tests have no external side effects,
  - PostgreSQL tests use temporary databases and cluster guards.
- If migration or adapter work fails midway:
  - revert the current migration and schema/model changes together,
  - re-run formatter/lint/tests before retrying.
- If behavioural tests fail due to stale generated step bindings:
  - clean build artifacts and rerun targeted scenario tests before broader
    suites.

## Artifacts and notes

Expected implementation touchpoints:

- Module wiring:
  - `src/lib.rs`
  - `src/task/mod.rs` (new)
- Task domain/ports/services:
  - `src/task/domain/*.rs` (new)
  - `src/task/ports/*.rs` (new)
  - `src/task/services/*.rs` (new)
- Adapters:
  - `src/task/adapters/memory/*.rs` (new)
  - `src/task/adapters/postgres/*.rs` (new)
- Persistence:
  - `migrations/<new_task_migration>/up.sql`
  - `migrations/<new_task_migration>/down.sql`
  - `src/message/adapters/schema.rs` (or task schema file if split)
  - `src/message/adapters/models.rs` (or task models file if split)
- Tests:
  - `src/task/tests/*.rs` and `tests/in_memory/*` (as applicable)
  - `tests/postgres/*task*`
  - `tests/features/task_issue_creation.feature`
  - `tests/task_issue_creation_steps.rs` (or folder module)
- Documentation:
  - `docs/corbusier-design.md`
  - `docs/users-guide.md`
  - `docs/roadmap.md`

Keep command logs captured under `/tmp/1-2-1-*.log` for review during execution.

## Interfaces and dependencies

Prescriptive interfaces for this milestone:

- Domain types:
  - `TaskId`, `Task`, `TaskState`, `TaskOrigin`, `IssueRef`,
    `IssueMetadataSnapshot`.
- Port contracts:
  - repository/service methods to create from issue metadata and to retrieve by
    canonical issue reference.
- Service orchestration:
  - `create_from_issue(input) -> Result<Task, TaskError>`
  - `find_by_issue_ref(issue_ref) -> Result<Option<Task>, TaskError>`
- Adapter behaviour:
  - in-memory adapter implements full port semantics for fast tests,
  - PostgreSQL adapter enforces uniqueness and query-by-origin semantics with
    SQL index support.
- Dependencies:
  - `rstest = 0.26.x` (existing),
  - `rstest-bdd = 0.5.0`,
  - `rstest-bdd-macros = 0.5.0`,
  - `pg-embed-setup-unpriv = 0.5.0`.

The interface and adapter signatures must remain domain-centric and must not
leak Diesel or transport types across port boundaries.

## Revision note

Initial draft created for roadmap item 1.2.1 based on repository state and
design references dated 2026-02-09.
