# Implement MCP server lifecycle management (roadmap 2.1.1)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

`PLANS.md` is not present in this repository as of 2026-02-27, so this plan is
the controlling execution document for roadmap item 2.1.1.

## Purpose / big picture

Implement roadmap item 2.1.1 so Corbusier can host MCP servers with explicit
lifecycle operations (`start`, `stop`, health reporting), register those
servers in a tool registry, and query tools exposed by running servers.

After this change, a developer or operator can:

1. Register MCP server configurations in the tool registry.
2. Start and stop registered MCP servers through a service API.
3. List registered servers with lifecycle state and last health status.
4. Query tools from a running server (`tools/list`) through the registry.

Observable success: `make all`, `make markdownlint`, and `make nixie` pass, and
new unit (`rstest`), behavioural (`rstest-bdd`), in-memory integration, and
PostgreSQL integration tests prove happy paths, unhappy paths, and key edge
cases.

## Constraints

- Keep strict hexagonal boundaries:
  - Domain code is infrastructure-agnostic (no Diesel, process, or transport
    implementation details in domain types).
  - Port traits are defined in the `tool_registry` core module.
  - Adapters implement ports; adapters do not depend on one another directly.
- Scope strictly to roadmap 2.1.1:
  - Include lifecycle management, registration, and tool listing/query support.
  - Do not implement tool-call execution routing/policy logic from 2.1.2.
- Preserve existing public behaviour in `message`, `task`, and `agent_backend`.
- Add module-level `//!` docs and public Rustdoc comments for all new modules
  and public APIs.
- Avoid `unsafe` code.
- Keep files below the 400-line repository rule by splitting modules early.
- Use `rstest` for unit/integration fixtures and `rstest-bdd` for behavioural
  scenarios.
- Use existing `pg-embed-setup-unpriv` harness patterns for PostgreSQL tests.
- Update docs as part of the feature:
  - Record implementation decisions in `docs/corbusier-design.md`.
  - Update `docs/users-guide.md` for user-visible lifecycle/registry behaviour.
  - Mark roadmap 2.1.1 and its sub-items done in `docs/roadmap.md` only after
    all quality gates pass.

## Tolerances (exception triggers)

- Scope: if the implementation needs more than 35 files changed or 2,400 net
  lines, stop and escalate with a reduced-scope option.
- API surface: if implementing 2.1.1 requires incompatible changes to existing
  public APIs outside the new module, stop and escalate.
- Dependencies: if a new external crate is required, stop and escalate with
  rationale and alternatives.
- Transport ambiguity: if `start`/`stop` semantics for HTTP+SSE cannot be made
  coherent without changing roadmap scope, stop and escalate with options.
- Iterations: if a failing test remains unresolved after 4 focused
  fix-and-rerun cycles, stop and document alternatives.
- Milestone duration: if any single implementation stage exceeds 4 hours of
  active work, stop and report remaining unknowns.

## Risks

- Risk: MCP protocol integration can expand into 2.1.2 routing concerns.
  Severity: high Likelihood: medium Mitigation: confine this feature to
  lifecycle + registry + `tools/list`; keep tool-call execution routing
  deferred.

- Risk: Lifecycle process management tests may become flaky if spawned child
  processes are not isolated and terminated deterministically. Severity: high
  Likelihood: medium Mitigation: use deterministic test doubles for unit tests;
  for integration, enforce explicit teardown and idempotent `stop`.

- Risk: New PostgreSQL schema may be omitted from `tests/postgres/helpers.rs`,
  causing false negatives or mismatched test templates. Severity: medium
  Likelihood: medium Mitigation: add migration SQL constant and apply it in
  `apply_migrations()` alongside existing migration constants.

- Risk: Behaviour test module naming can conflict with Rust module resolution.
  Severity: low Likelihood: medium Mitigation: keep scenario runner filename
  distinct from step-definition directory (for example,
  `mcp_server_lifecycle_scenarios.rs` vs `mcp_server_lifecycle_steps/`).

## Progress

- [x] (2026-02-27 00:00Z) Gathered roadmap/design requirements for 2.1.1.
- [x] (2026-02-27 00:00Z) Mapped existing hexagonal module and test patterns.
- [x] (2026-02-27 00:00Z) Authored initial ExecPlan draft.
- [x] (2026-02-28 00:00Z) Received approval and implemented Stage A scaffolding
      under `src/tool_registry/` with domain, ports, adapters, and services.
- [x] (2026-02-28 00:00Z) Added additive migration
      `migrations/2026-02-28-000000_add_mcp_servers_table/` and wired it into
      `tests/postgres/helpers.rs`.
- [x] (2026-02-28 00:00Z) Added unit tests (`rstest`), in-memory integration
      tests, behavioural tests (`rstest-bdd`), and PostgreSQL integration tests
      for lifecycle and tool-query behaviour.
- [x] (2026-02-28 00:00Z) Implemented lifecycle service operations:
      `register`, `start`, `stop`, `refresh_health`, `list_all`,
      `find_by_name`, and `list_tools`.
- [x] (2026-02-28 00:00Z) Ran core code gates successfully:
      `make check-fmt`, `make lint`, and `make test`.
- [x] (2026-02-28 00:00Z) Run documentation gates after final doc updates:
      `make fmt`, `make markdownlint`, and `make nixie`.

## Surprises & Discoveries

- Observation: No `tool_registry` or MCP lifecycle module exists yet; this is a
  new bounded context. Evidence: source tree inspection under `src/`. Impact:
  2.1.1 must establish domain, ports, adapters, services, tests, and
  persistence from scratch.

- Observation: PostgreSQL integration tests use explicit migration inclusion in
  `tests/postgres/helpers.rs` rather than automatic migration discovery.
  Evidence: helper constants and `apply_migrations()` sequence. Impact: the new
  migration must be added to that helper, not only to `migrations/`.

- Observation: BDD structure for this feature worked cleanly as a single
  `tests/mcp_server_lifecycle_steps.rs` file with `#[scenario(...)]` entries,
  rather than split `*_steps/` directory + scenario runner file. Evidence:
  `rstest-bdd` scenarios compile and execute from one file. Impact: this
  avoided module naming collisions and reduced boilerplate.

## Decision Log

- Decision: introduce a dedicated `tool_registry` bounded context under
  `src/tool_registry/` rather than extending `agent_backend`. Rationale: MCP
  server lifecycle and tool registry concerns are distinct from agent backend
  identity/capability registration and map directly to roadmap section 2.1.
  Date/Author: 2026-02-27 / plan author

- Decision: model lifecycle orchestration through two ports:
  `McpServerRegistryRepository` (persistence) and `McpServerHost` (runtime
  transport/process control). Rationale: this preserves hexagonal separation
  and keeps runtime concerns out of domain and persistence adapters.
  Date/Author: 2026-02-27 / plan author

- Decision: scope 2.1.1 to `tools/list` querying only, with `tools/call`
  execution routing deferred to 2.1.2. Rationale: matches roadmap sequencing
  and prevents accidental scope creep into policy/routing concerns.
  Date/Author: 2026-02-27 / plan author

- Decision: persist transport configuration in PostgreSQL as JSONB
  (`transport_config`) and lifecycle/health fields as scalar columns.
  Rationale: supports both stdio and HTTP+SSE without frequent schema churn
  while keeping filterable state and health metadata explicit. Date/Author:
  2026-02-28 / implementation

- Decision: use `InMemoryMcpServerHost` as the runtime host adapter in test
  coverage, including PostgreSQL integration tests. Rationale: keeps tests
  deterministic while validating persistence and service behaviour
  independently from external process management concerns. Date/Author:
  2026-02-28 / implementation

## Outcomes & Retrospective

Implemented outcome:

- Added `tool_registry` bounded context with strict hexagonal separation:
  domain model, ports, adapters (memory/postgres/runtime host), and service
  orchestration.
- Added MCP server lifecycle support for registration, start, stop, health
  refresh/reporting, and tool listing from running servers.
- Added persistence via an additive `mcp_servers` migration and PostgreSQL
  repository adapter.
- Added test coverage across unit, behavioural, in-memory integration, and
  PostgreSQL integration layers for happy and unhappy paths.

Lessons learned:

- Keeping runtime lifecycle in a host port and persistence in a repository port
  prevented adapter leakage into the domain model.
- Behaviour scenarios remained easier to maintain when colocated with step
  functions in a single file for this feature.

## Context and orientation

Corbusier currently has completed bounded contexts for `message`, `task`, and
`agent_backend`, each following the same hexagonal shape: `domain/`, `ports/`,
`adapters/{memory,postgres}/`, `services/`, plus unit tests under
`src/<context>/tests/`.

Integration and behavioural test patterns already established:

- In-memory integration modules aggregated in `tests/in_memory.rs`.
- PostgreSQL integration modules aggregated in `tests/postgres.rs`.
- BDD files under `tests/features/`, with step modules in
  `tests/*_steps/` and scenario runners in `tests/*_scenarios.rs` or
  `tests/*_steps.rs`.

PostgreSQL harness expectations:

- Use `pg-embed-setup-unpriv` through `tests/postgres/cluster/` fixtures.
- Ensure template database migrations in `tests/postgres/helpers.rs`.

Reference documents for this feature:

- `docs/roadmap.md` (2.1.1 scope and success criteria).
- `docs/corbusier-design.md` §2.2.4 and §6.1.4 (MCP lifecycle and tool
  registry architecture).
- `docs/rust-testing-with-rstest-fixtures.md`.
- `docs/reliable-testing-in-rust-via-dependency-injection.md`.
- `docs/rust-doctest-dry-guide.md`.
- `docs/complexity-antipatterns-and-refactoring-strategies.md`.
- `docs/pg-embed-setup-unpriv-users-guide.md`.
- `docs/ortho-config-users-guide.md`.
- `docs/rstest-bdd-users-guide.md`.

## Plan of work

### Stage A: Design lock and scaffolding (no behaviour change outside new context)

Create `src/tool_registry/` with hexagonal structure:

- `src/tool_registry/domain/`:
  - server identity/value objects (`McpServerId`, `McpServerName`),
  - transport configuration (`McpTransport`),
  - lifecycle state and health snapshot types,
  - server registration aggregate with timestamped state transitions.
- `src/tool_registry/ports/`:
  - `McpServerRegistryRepository` for persistence and lookup,
  - `McpServerHost` for runtime `start`, `stop`, `health`, and `list_tools`.
- `src/tool_registry/services/`:
  - lifecycle orchestration service and request DTOs.
- `src/tool_registry/adapters/`:
  - in-memory repository adapter,
  - PostgreSQL repository adapter,
  - host adapter(s) for runtime lifecycle integration.

Wire module exports in `src/lib.rs` and `src/tool_registry/mod.rs`.

Add additive migration for MCP server registry persistence (for example
`mcp_servers` with transport config, lifecycle state, and health/tool snapshot
fields) and include it in `tests/postgres/helpers.rs`.

Go/no-go checkpoint: do not proceed until scaffolding compiles and module
boundaries are clean.

### Stage B: Tests first (red phase)

Add failing tests that define required 2.1.1 behaviour.

Unit tests with `rstest`:

- Domain validation and state transition tests:
  - valid server configuration and registration,
  - invalid/empty server names or transport fields,
  - lifecycle transition legality (`registered -> running -> stopped`),
  - health status update semantics.
- Service tests with mocked `McpServerHost`/repository ports:
  - start/stop happy path,
  - start unknown server,
  - start host failure surfaces typed error,
  - tool query requires running/healthy server.

In-memory integration tests:

- Add `tests/in_memory/mcp_server_lifecycle_tests.rs` and register it in
  `tests/in_memory.rs`.
- Cover end-to-end service flow against in-memory adapters:
  register, start, list, health, tools query, stop.

Behaviour tests with `rstest-bdd`:

- Add `tests/features/mcp_server_lifecycle.feature`.
- Add scenario/step runner `tests/mcp_server_lifecycle_steps.rs`.
- Cover happy and unhappy paths:
  - starting a registered server,
  - duplicate registration rejection,
  - health reporting after start/stop,
  - querying tools from a running server.

PostgreSQL integration tests:

- Add `tests/postgres/mcp_server_lifecycle_tests.rs`.
- Register the module in `tests/postgres.rs`.
- Verify persisted lifecycle state, health snapshot, and server listing/query
  behaviour against real PostgreSQL.

Go/no-go checkpoint: proceed only when new tests fail for expected missing
implementation reasons.

### Stage C: Implementation (green phase)

Implement domain and service logic in minimal increments to satisfy Stage B
tests.

- Implement domain constructors, invariant checks, and lifecycle transitions.
- Implement in-memory repository adapter.
- Implement PostgreSQL adapter:
  - schema/models/repository conversion helpers,
  - duplicate-name handling,
  - reconstruction validation with typed persistence errors.
- Implement host adapter logic for lifecycle and `tools/list` querying, mapped
  behind `McpServerHost`.
- Implement service methods for:
  - register server,
  - start server,
  - stop server,
  - list servers with health,
  - query available tools from a server.

Keep functions small and split modules before complexity grows (use helper
functions for transition predicates and transport-specific behaviour).

Go/no-go checkpoint: proceed only when all new unit, behavioural, in-memory,
and PostgreSQL tests pass.

### Stage D: Hardening, documentation, and roadmap completion

- Add or refine Rustdoc examples for new public APIs where useful.
- Update `docs/corbusier-design.md` by appending a dated
  "Implementation decisions" subsection under the F-005 / 2.2.4 context.
- Update `docs/users-guide.md` with lifecycle and registry usage examples
  (register/start/list/health/tools/stop).
- Mark `docs/roadmap.md` item 2.1.1 and its sub-bullets as done.
- Run full quality gates and documentation validators.

Go/no-go checkpoint: feature is complete only when code and documentation gates
all pass and roadmap status is updated.

## Concrete steps

Run all commands from repository root: `/home/user/project`.

1. Baseline and scaffolding checks during development:

   ```bash
   set -o pipefail && make check-fmt 2>&1 | tee /tmp/2-1-1-check-fmt.log
   set -o pipefail && make lint 2>&1 | tee /tmp/2-1-1-lint.log
   ```

   Expected signal: no formatting/lint regressions while scaffolding evolves.

2. Fast iteration on targeted tests while implementing:

   ```bash
   set -o pipefail && cargo nextest run --all-targets --all-features mcp_server 2>&1 | tee /tmp/2-1-1-targeted-tests.log
   ```

   Expected signal: new MCP lifecycle suites fail first (red), then pass
   (green) as implementation lands.

3. Full repository gates before completion:

   ```bash
   set -o pipefail && make all 2>&1 | tee /tmp/2-1-1-make-all.log
   set -o pipefail && PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/2-1-1-markdownlint.log
   set -o pipefail && make nixie 2>&1 | tee /tmp/2-1-1-nixie.log
   ```

   Expected signal: all commands exit 0, with no lint, test, markdown, or
   Mermaid validation failures.

## Validation and acceptance

Acceptance is behavioural:

1. A registered MCP server can be started and transitions to a running state.
2. Listing servers returns registered entries with lifecycle and health data.
3. Querying available tools from a running server returns tool definitions.
4. Stopping a running server updates lifecycle/health state and removes runtime
   availability.
5. Duplicate registrations, unknown server IDs, and host start/query failures
   return typed errors and are covered by tests.

Quality criteria:

- Tests: all existing tests and new 2.1.1 tests pass.
- Lint/format: `make check-fmt`, `make lint`, and `make all` pass.
- Docs validation: `make markdownlint` and `make nixie` pass after doc updates.

## Idempotence and recovery

- Test and verification commands are safe to rerun.
- Migration is additive; if migration/testing fails, recreate temporary
  databases via existing PostgreSQL test harness.
- Lifecycle operations should be coded idempotently where practical:
  - starting an already running server should be a typed no-op or typed
    conflict,
  - stopping an already stopped server should not leave inconsistent state.

## Interfaces and dependencies

Planned core interfaces (names may be refined during implementation but must
preserve responsibility boundaries):

```rust
#[async_trait::async_trait]
pub trait McpServerRegistryRepository: Send + Sync {
    async fn register(&self, server: &McpServerRegistration) -> Result<(), ToolRegistryError>;
    async fn update(&self, server: &McpServerRegistration) -> Result<(), ToolRegistryError>;
    async fn find_by_id(&self, id: McpServerId) -> Result<Option<McpServerRegistration>, ToolRegistryError>;
    async fn find_by_name(&self, name: &McpServerName) -> Result<Option<McpServerRegistration>, ToolRegistryError>;
    async fn list_all(&self) -> Result<Vec<McpServerRegistration>, ToolRegistryError>;
}
```

```rust
#[async_trait::async_trait]
pub trait McpServerHost: Send + Sync {
    async fn start(&self, server: &McpServerRegistration) -> Result<(), ToolRegistryError>;
    async fn stop(&self, server: &McpServerRegistration) -> Result<(), ToolRegistryError>;
    async fn health(&self, server: &McpServerRegistration) -> Result<ServerHealthSnapshot, ToolRegistryError>;
    async fn list_tools(&self, server: &McpServerRegistration) -> Result<Vec<ToolDefinition>, ToolRegistryError>;
}
```

No new external dependency is assumed in this plan. If MCP transport handling
cannot be implemented cleanly with current dependencies, trigger tolerance
escalation and document options.

## Artifacts and notes

Implementation should capture concise evidence in this document during
execution (for example, key test-pass summaries and final gate command results)
so a future maintainer can verify completion without rerunning every step.

Validation evidence:

- `/tmp/2-1-1-check-fmt.log`: `make check-fmt` passed.
- `/tmp/2-1-1-lint.log`: `make lint` passed.
- `/tmp/2-1-1-test.log`: `make test` passed
  (`577 tests run: 577 passed, 1 skipped`).
- `/tmp/2-1-1-doc-fmt.log`: `make fmt` passed after resolving a duplicate
  heading (`MD024`) in `docs/corbusier-design.md`.
- `/tmp/2-1-1-markdownlint.log`: `make markdownlint` passed.
- `/tmp/2-1-1-nixie.log`: `make nixie` passed.

## Revision note

2026-02-27: Initial draft created from roadmap 2.1.1, design sections 2.2.4 and
6.1.4, and current repository testing/architecture conventions. 2026-02-28:
Implementation completed for roadmap 2.1.1 with module, migration, tests, and
service delivery. Documentation and roadmap updates applied. 2026-02-28: Full
gates completed successfully (`make check-fmt`, `make lint`, `make test`,
`make fmt`, `make markdownlint`, `make nixie`).
