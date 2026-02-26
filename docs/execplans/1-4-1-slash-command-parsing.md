# Deliver slash command parsing and template execution (roadmap 1.4.1)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

This document defines implementation for roadmap item 1.4.1 in
`docs/roadmap.md`:

- Implement command parser and registry.
- Add template expansion and parameter validation using `minijinja`.
- Achieve deterministic tool call sequences with auditable records.

Execution phase was approved and completed in this implementation turn.

## Purpose / big picture

After this change, Corbusier will accept slash commands (for example,
`/task action=start issue=123`), validate and expand them through versioned
templates, and produce deterministic tool call plans that are recorded in
message metadata for auditability.

Observable outcome for operators and developers:

1. The same command input and parameter set always produce the same ordered
   tool call sequence.
2. Invalid commands are rejected with typed errors that identify why parsing or
   validation failed.
3. Expanded command content and planned tool calls are persisted in the
   canonical message/audit structures, and are queryable through existing
   history retrieval paths.

This plan uses the existing message subsystem as the implementation home so
slash-command execution remains close to canonical message and audit metadata
(`SlashCommandExpansion`, `ToolCallAudit`).

## Constraints

- Keep strict hexagonal boundaries:
  - Domain logic (parse, validation rules, deterministic planning) stays in
    `src/message/domain/` and `src/message/services/` with no infrastructure
    imports.
  - Ports in `src/message/ports/` define registry/loading contracts.
  - Adapters in `src/message/adapters/` implement those ports only.
- Do not regress existing message and task behaviour covered by current tests.
- Use `minijinja` for template rendering and parameter interpolation.
- Use `rstest` for unit tests and `rstest-bdd` for behavioural tests.
- Cover happy paths, unhappy paths, and deterministic-order edge cases.
- Use `pg-embed-setup-unpriv` fixtures for Postgres-backed integration tests.
- Keep each Rust file under 400 lines.
- Maintain en-GB-oxendict spelling in docs and comments.
- Update documentation deliverables as part of feature completion:
  - `docs/corbusier-design.md` design decisions for 1.4.1.
  - `docs/users-guide.md` user-visible slash command behaviour.
  - `docs/roadmap.md` mark 1.4.1 done only after all validation gates pass.

## Tolerances (exception triggers)

- Scope tolerance: stop and escalate if implementation requires more than
  18 files or 1,600 net new lines.
- Interface tolerance: stop and escalate if existing public message/task API
  signatures must change in incompatible ways.
- Dependency tolerance: stop and escalate if more than one new external crate
  beyond `minijinja` is required.
- Data tolerance: stop and escalate if the feature requires a new database
  migration; 1.4.1 should fit the existing message metadata model.
- Iteration tolerance: stop and escalate if the same failure persists after
  three focused fix attempts.
- Ambiguity tolerance: stop and escalate if deterministic ordering semantics
  remain ambiguous after reviewing `docs/corbusier-design.md` and existing
  metadata models.

## Risks

- Risk: Parser grammar grows too broad and creates ambiguous parses.
  Severity: medium. Likelihood: medium. Mitigation: start with a constrained
  grammar (`/<command> key=value ...`), provide typed parse errors, and add
  parameterized parser tests.

- Risk: Nondeterministic ordering from unordered maps leaks into tool planning.
  Severity: high. Likelihood: medium. Mitigation: normalize parameter handling
  before planning and generate deterministic call IDs from canonicalized inputs.

- Risk: Template rendering errors surface late and produce partial audit data.
  Severity: medium. Likelihood: medium. Mitigation: validate command and
  parameters fully before rendering; render in a fail-fast service path that
  returns typed errors and emits no tool plan on failure.

- Risk: Behavioural test wiring conflicts with existing `rstest-bdd` naming.
  Severity: low. Likelihood: medium. Mitigation: use a dedicated scenario
  entrypoint filename that differs from the step-definition directory name.

## Progress

- [x] (2026-02-26 17:34Z) Gathered roadmap/design/testing constraints and
      drafted this ExecPlan.
- [x] (2026-02-26 18:08Z) Stage A: Confirmed command grammar, registry
      contract, and deterministic planning rules.
- [x] (2026-02-26 18:15Z) Stage B: Implemented domain, port, service, and
      adapter code for parser, registry, expansion, and deterministic planning.
- [x] (2026-02-26 18:18Z) Stage C: Added unit, integration, and BDD coverage
      for happy/unhappy/edge cases.
- [x] (2026-02-26 18:25Z) Stage D: Updated user/design docs, passed quality
      gates, and marked roadmap item 1.4.1 complete.

## Surprises & discoveries

- `docs/corbusier-design.md` section 2.1.1 defines F-004 at feature level but
  does not provide a dedicated `F-004-RQ-*` requirement table comparable to
  F-001/F-002/F-003; concrete acceptance behaviour must therefore be anchored
  to the slash-command interface description and conversation component model
  (`SlashCommand`, `SlashCommandExpansion`, and `execute_slash_command` port
  sketch).
- The repository already stores slash expansion metadata in
  `MessageMetadata::slash_command_expansion`, so 1.4.1 can remain
  migration-free unless a hard schema gap is discovered during implementation.
- `minijinja` in this environment did not expose a `tojson` filter in template
  rendering defaults; built-in command templates were adjusted to emit valid
  JSON without relying on that filter.
- Clippy guardrails (`cognitive_complexity`, `excessive_nesting`,
  `indexing_slicing`, and strict docs) required parser/service refactors and
  tighter test annotations during implementation.

## Decision log

- Decision: Implement slash command orchestration inside the message subsystem
  rather than creating a third top-level feature module. Rationale: slash
  expansion and audit data already live in message-domain metadata, and
  conversation orchestration in the design identifies slash handling as part of
  the conversation component. Date/Author: 2026-02-26 / plan author.

- Decision: Treat deterministic output as an explicit contract of the slash
  service, verified by repeat-execution tests. Rationale: roadmap 1.4.1 success
  criteria explicitly require deterministic tool call sequences with auditable
  records. Date/Author: 2026-02-26 / plan author.

- Decision: Keep 1.4.1 storage model within existing message metadata and tool
  audit fields. Rationale: this minimizes blast radius and avoids unnecessary
  persistence migrations while still satisfying auditability requirements.
  Date/Author: 2026-02-26 / plan author.

- Decision: Keep built-in tool argument templates JSON-safe without
  `tojson`-filter dependence. Rationale: this keeps runtime rendering portable
  across `minijinja` filter configurations while preserving deterministic
  output. Date/Author: 2026-02-26 / implementation author.

## Outcomes & retrospective

Roadmap 1.4.1 was implemented with the following outcomes:

- Added slash-command domain model, parser, typed validation, execution output,
  and error types under `src/message/domain/slash_command/`.
- Added `SlashCommandRegistry` port and an in-memory registry adapter with
  built-in `/task` and `/review` definitions.
- Added `SlashCommandService` with parse -> lookup -> validate -> render ->
  deterministic call-id planning -> audit record generation.
- Added unit tests (`rstest`), in-memory integration tests, PostgreSQL
  integration tests (using existing `pg-embed-setup-unpriv` fixture pipeline),
  and BDD scenarios (`rstest-bdd`).
- Updated `docs/users-guide.md`, `docs/corbusier-design.md`, and
  `docs/roadmap.md` (marked 1.4.1 done).

Validation evidence:

- `make check-fmt` passed.
- `make lint` passed.
- `make test` passed (514 tests, 514 passed, 1 skipped).

Lessons learned:

- Treat deterministic sequencing as a first-class contract and test it at both
  unit and behavioural levels.
- Keep slash-command templates intentionally simple and JSON-explicit to avoid
  runtime filter coupling.

## Context and orientation

Current relevant code and docs:

- `docs/roadmap.md` marks 1.4.1 as complete.
- `docs/corbusier-design.md` section 2.1.1 defines F-004 and section 6.1.1
  models slash command execution within the conversation component.
- `src/message/domain/metadata.rs` already includes `SlashCommandExpansion`
  and tool-call audit metadata.
- Slash-command parser/registry/template-execution now exists in
  `src/message/domain/slash_command/`, `src/message/ports/slash_command.rs`,
  `src/message/adapters/memory/slash_command.rs`, and
  `src/message/services/slash_command.rs`.
- `tests/in_memory.rs` and `tests/postgres.rs` are the integration test module
  entrypoints that must register new test modules.
- Existing BDD style is split between single-file scenarios and directory-based
  step modules (for complex features).

Implementation orientation:

1. Extend message-domain capability with slash command types and errors.
2. Add a command registry port and a default in-memory adapter.
3. Add an application service that performs parse -> lookup -> validate ->
   render -> deterministic tool-call planning -> audit record assembly.
4. Validate this behaviour in unit tests (`rstest`), integration tests, and
   feature scenarios (`rstest-bdd`).

## Plan of work

### Stage A: Domain and contract design (no behavioural changes yet)

Define a constrained slash-command model and parser contract:

- Add domain types for:
  - parsed invocation,
  - command definition and parameter schema,
  - planned tool call sequence,
  - typed command errors.
- Define command grammar and normalization rules:
  - leading slash required,
  - command token,
  - key-value parameters,
  - duplicate-key handling,
  - unknown-parameter handling.
- Add a registry port that returns command definitions by command name.

Go/no-go: proceed only when parser outputs and error semantics are fully typed
and testable without adapters.

### Stage B: Parser, registry, template rendering, and deterministic planning

Implement slash-command orchestration service:

- Parse raw input into a normalized invocation.
- Resolve command definition from registry.
- Validate required and typed parameters.
- Render expanded content through `minijinja`.
- Produce deterministic tool call sequence from command definition and
  normalized parameters.
- Build auditable metadata payloads:
  - `SlashCommandExpansion` with command + parameters + rendered content.
  - ordered tool call audit records tied to planned tool calls.

Planned code touch-points:

- `src/message/domain/` (new slash command domain modules).
- `src/message/ports/` (new slash command registry port).
- `src/message/services/` (new slash command service).
- `src/message/adapters/memory/` (registry adapter).
- `src/message/mod.rs` and module export files.

Go/no-go: proceed only when service can run in-memory and produce deterministic
plans for repeated identical input.

### Stage C: Test coverage

Unit tests (`rstest`) for:

- parser success and failure cases,
- parameter validation (required/type/select),
- template rendering success/failure via `minijinja`,
- deterministic sequence and call ID stability.

Integration tests for:

- in-memory end-to-end slash execution workflow,
- Postgres persistence round-trip of expansion and audit metadata.

Behavioural tests (`rstest-bdd`) for:

- valid slash command expansion producing expected tool plan,
- unknown command rejection,
- missing/invalid parameter rejection,
- repeated execution determinism.

Postgres test support:

- Use `pg_embedded_setup_unpriv::test_support::{shared_test_cluster,...}`
  patterns already present in `tests/postgres/helpers.rs`.
- Reuse template database setup via helper functions to keep tests fast and
  isolated.

Go/no-go: proceed only when new tests fail before implementation and pass after
implementation.

### Stage D: Documentation, roadmap, and hardening

- Update `docs/users-guide.md` with slash command usage, validation failures,
  and deterministic audit behaviour.
- Add implementation decisions for roadmap 1.4.1 to
  `docs/corbusier-design.md`.
- Mark roadmap item 1.4.1 and its sub-items complete in `docs/roadmap.md`
  after all quality gates pass.
- Run all required formatting, linting, tests, and markdown validations.

## Concrete steps

All commands run from repository root (`/home/user/project`).

1. Create feature branches of code changes in this order:
   domain -> ports -> adapters/services -> tests -> docs.
2. Run targeted tests after each stage and keep logs:

   ```bash
   set -o pipefail && make test 2>&1 | tee /tmp/1-4-1-stage-test.log
   ```

3. Ensure Postgres test harness tooling is available:

   ```bash
   command -v pg_embedded_setup_unpriv >/dev/null || cargo install pg-embed-setup-unpriv
   ```

4. Run full code quality gates once implementation is complete:

   ```bash
   set -o pipefail && make check-fmt 2>&1 | tee /tmp/1-4-1-check-fmt.log
   set -o pipefail && make lint 2>&1 | tee /tmp/1-4-1-lint.log
   set -o pipefail && make test 2>&1 | tee /tmp/1-4-1-test.log
   ```

5. Run documentation gates after docs updates:

   ```bash
   set -o pipefail && make fmt 2>&1 | tee /tmp/1-4-1-fmt.log
   set -o pipefail && PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/1-4-1-markdownlint.log
   set -o pipefail && make nixie 2>&1 | tee /tmp/1-4-1-nixie.log
   ```

Expected success transcript shape:

```plaintext
... check-fmt: exit 0
... lint: exit 0
... test: all tests passed
... markdownlint: 0 errors
... nixie: All diagrams validated successfully
```

## Validation and acceptance

Behavioural acceptance:

1. Given a registered command and valid parameters, execution returns expanded
   content and a non-empty planned tool-call sequence.
2. Given the same input repeated, planned tool calls (order and IDs) are
   identical across executions.
3. Given an unknown command, execution returns a typed unknown-command error.
4. Given missing or invalid parameters, execution returns typed validation
   errors and no planned tool-call sequence.
5. Persisted message history contains slash expansion and tool-call audit data
   for successful executions.

Quality acceptance:

- `make check-fmt` passes.
- `make lint` passes with `-D warnings`.
- `make test` passes across workspace tests, including new unit/integration/BDD
  cases.
- `make markdownlint` and `make nixie` pass after documentation changes.

## Idempotence and recovery

- Parser and service steps are idempotent at code-generation level and safe to
  rerun.
- Test commands are safe to rerun and should produce consistent pass/fail
  status.
- If a stage fails, fix only that stage, rerun its targeted command, then rerun
  full quality gates.
- If a tolerance threshold is crossed, stop and record escalation options in
  `Decision Log` before continuing.

## Artifacts and notes

Expected new/updated artifact groups:

- Slash command domain/ports/services/adapters in `src/message/...`.
- Unit tests under `src/message/tests/...`.
- Integration tests under `tests/in_memory/...` and `tests/postgres/...`.
- BDD files under `tests/features/` and `tests/slash_command_*`.
- Documentation updates in `docs/users-guide.md`,
  `docs/corbusier-design.md`, and `docs/roadmap.md`.

Store command logs under `/tmp/1-4-1-*.log` for auditable implementation
evidence.

## Interfaces and dependencies

New dependency expectation:

- `minijinja` added to `Cargo.toml` (caret requirement).

Planned new interfaces (exact names may be finalized during Stage A):

- Domain:
  - `SlashCommandInvocation`
  - `SlashCommandDefinition`
  - `CommandParameterSpec`
  - `PlannedToolCall`
  - `SlashCommandError`
- Port:
  - `SlashCommandRegistry`
- Service:
  - `SlashCommandService::execute(...)` returning rendered content plus ordered
    planned tool calls and metadata suitable for message/audit persistence.

No external services beyond existing repository dependencies are required for
1.4.1.
