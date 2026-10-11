# Translate tool schemas per backend (roadmap 2.3.3)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

Execution phase does not begin until this draft is explicitly approved.

## Purpose / big picture

Roadmap item 2.3.2 gave Corbusier an `AgentTurnOrchestratorService` that can
run a turn, reuse or rotate backend sessions, and route emitted tool calls
through one canonical path. Roadmap item 3.1.2 gave Corbusier a durable Model
Context Protocol (MCP) tool catalogue built around `McpToolDefinition`.

What is still missing is the bridge between those two completed areas. Backends
can execute turns, and tools can be discovered and routed, but the orchestrator
still has no way to present the discovered canonical MCP tool schemas in the
backend-specific format each agent expects. That gap is why
`docs/corbusier-design.md` still lists tool schema translation as a first-class
agent-backend requirement.

After this change, a caller can register a backend, discover tools once through
the canonical MCP catalogue, and execute turns without writing backend-specific
tool schema glue by hand. Corbusier translates the canonical tool definitions
for the chosen backend automatically, validates that translation, and passes
the translated tool surface into the runtime adapter before the turn begins.

Observable outcomes:

1. A backend turn can be executed with a discovered canonical tool definition,
   and the backend receives an accepted translated schema without manual edits.
2. The same canonical MCP tool definition can be translated for each supported
   backend through one orchestrated path.
3. Unsupported or lossy schema shapes fail early with typed errors before the
   runtime starts processing the turn.

## Reference documents and skills

The implementer should keep these documents open while working:

- `docs/roadmap.md` item `2.3.3` for delivery scope and done criteria.
- `docs/corbusier-design.md` section `2.2.3` for the tool schema translation
  requirement and section `2.2.4` for the canonical MCP tool model.
- `docs/corbusier-design.md` section `6.1.3` for the target
  `AgentBackend::translate_tool_schema` shape and the backend comparison table.
- `docs/execplans/2-3-1-agent-backend-registration-and-discovery.md` for the
  existing backend registration model.
- `docs/execplans/2-3-2-orchestrate-agent-turn-execution-and-sessions.md` for
  the current orchestration seam and its explicit deferral of schema
  translation.
- `docs/execplans/3-1-2-tool-discovery-and-routing.md` for the delivered MCP
  tool catalogue, validation, and audit path.
- `docs/rust-testing-with-rstest-fixtures.md` for `rstest` fixture structure.
- `docs/rstest-bdd-users-guide.md` for Behaviour-Driven Development (BDD)
  scenario layout and fixture naming rules.
- `docs/reliable-testing-in-rust-via-dependency-injection.md` for clock and
  environment isolation patterns.
- `docs/pg-embed-setup-unpriv-users-guide.md` for local PostgreSQL test setup
  and fixture usage.
- `docs/rust-doctest-dry-guide.md` for any public API example updates.
- `docs/complexity-antipatterns-and-refactoring-strategies.md` for keeping the
  orchestration changes out of bumpy-road territory.
- `docs/ortho-config-users-guide.md` only if this feature introduces new
  operator-facing configuration, which it should avoid unless a concrete need
  emerges during implementation.
- `docs/users-guide.md` because the feature changes user-visible backend-tool
  behaviour and will need a guide update on completion.

The implementer should also signpost these skills while working:

- `execplans` for keeping this file current during implementation.
- `hexagonal-architecture` for preserving the boundary between
  `agent_backend`, `tool_registry`, and concrete runtime adapters.
- `leta` for semantic code navigation before editing Rust symbols.
- `rust-router` for picking the smallest relevant Rust skill when deeper design
  questions appear.
- `rust-types-and-apis` for shaping translation DTOs and trait contracts.
- `rust-errors` for typed translation and validation failures.
- `nextest` for understanding the repository test runner behaviour when using
  `make test`.

## Context and orientation

The repository already contains the two halves that 2.3.3 must connect:

- `src/tool_registry/domain/tool.rs` defines the canonical `McpToolDefinition`
  shape: `name`, `description`, `input_schema`, and optional `output_schema`.
- `src/tool_registry/services/discovery/mod.rs` owns the canonical discovery
  and routing service for MCP tools.
- `src/agent_backend/services/orchestrator/mod.rs` owns backend turn
  orchestration.
- `src/agent_backend/ports/runtime.rs` currently exposes
  `create_session`, `teardown_session`, and `execute_turn`, but no translation
  method.
- `src/agent_backend/domain/turn.rs` currently defines
  `TurnExecutionRequest` as `conversation_id`, `prompt`, and emitted
  `tool_calls` only. There is no field or sibling type for translated available
  tools.
- `src/agent_backend/ports/tool_router.rs` routes tool calls, but it does not
  list or describe the canonical tool surface.

Repository search confirms that no translation symbol exists in `src/` today.
The only references to `translate_tool_schema` live in design or execplan
documentation.

This means 2.3.3 is not a small adapter tweak. It needs one new explicit
orchestration seam that:

1. obtains the canonical tool definitions that are already known to the tool
   registry,
2. translates them per backend,
3. validates the translation,
4. injects the translated tool surface into the runtime request, and
5. leaves canonical tool-call routing untouched.

The preferred boundary is:

- canonical tool definitions remain owned by the `tool_registry` bounded
  context,
- translation policy lives in `agent_backend`,
- backend-specific translation mechanics live behind an `agent_backend` port
  implemented by runtime adapters, and
- no new persistence is introduced unless implementation proves the absence of
  a stable in-memory mapping impossible.

## Constraints

- Preserve hexagonal architecture. Domain logic under `src/agent_backend/`
  must not import Diesel, process control, or concrete backend SDK types.
- Keep canonical MCP schema ownership in `tool_registry`. Do not move
  `McpToolDefinition` or duplicate the catalogue as a second source of truth.
- Do not bypass the canonical tool-call path already owned by
  `ToolRouterPort`; 2.3.3 adds translation for available tools, not a second
  call path.
- Prefer additive internal API changes over public API churn. In particular,
  keep `ExecuteAgentTurnRequest` stable if an internal runtime request type can
  carry the translated tool surface instead.
- Use `rstest` for unit and integration tests, and `rstest-bdd` for behavioural
  scenarios where the feature is observable at the service boundary.
- Use `pg-embedded-setup-unpriv` for PostgreSQL-backed integration coverage.
- Keep files under the repository 400-line limit by extracting helper modules
  early.
- Add module-level `//!` comments and public `///` docs with `# Errors`
  sections for new public items.
- Avoid new external dependencies unless a missing backend acceptance validator
  cannot be implemented with existing crates.
- Record any stable design decision in `docs/corbusier-design.md`.
- Update `docs/users-guide.md` for user-visible behaviour on completion.
- Mark roadmap item `2.3.3` done in `docs/roadmap.md` only after all quality
  gates pass.

## Tolerances (exception triggers)

- Scope: stop and escalate if the implementation needs more than 24 files
  changed or more than 1,600 net lines.
- Interface: stop and escalate if a public API outside `agent_backend` must
  change incompatibly.
- Dependency: stop and escalate if any new external crate is required.
- Data model: stop and escalate if a new persisted backend discriminator or a
  new database table becomes necessary to select the correct translator.
- Iterations: stop and escalate if one failure class survives 4 focused
  fix-and-rerun cycles.
- Ambiguity: stop and escalate if translation dispatch cannot be made stable
  from existing backend registration data (`BackendName`, `BackendInfo`,
  capabilities) without inventing a second identity system.

## Risks

- Risk: the current backend registration model does not expose a first-class
  `BackendKind`, so translation dispatch may be ambiguous if backend name and
  provider are insufficient. Severity: high Likelihood: medium Mitigation:
  prefer dispatch by normalized `BackendName` first; escalate only if real
  ambiguity appears.

- Risk: canonical MCP schemas may legally contain shapes that a supported
  backend cannot represent losslessly. Severity: high Likelihood: medium
  Mitigation: define the supported subset explicitly, reject unsupported shapes
  early with typed translation errors, and document the limitation in the
  design doc and user guide.

- Risk: adding translation steps directly inside
  `AgentTurnOrchestratorService` can create a large, hard-to-read service.
  Severity: medium Likelihood: high Mitigation: extract a dedicated translation
  helper or service module and keep the orchestrator focused on coordination.

- Risk: behavioural tests may become brittle if they assert backend-private
  payload details instead of service-visible outcomes. Severity: medium
  Likelihood: medium Mitigation: use strict fake runtime adapters that surface
  acceptance or rejection through typed results, then assert those outcomes at
  the orchestration boundary.

- Risk: PostgreSQL integration runs may time out under the default nextest
  profile. Severity: medium Likelihood: medium Mitigation: use the existing
  `long` profile via `TEST_FLAGS='--profile long --all-targets --all-features'`
  for final validation.

## Progress

- [x] (2026-04-13 00:00Z) Reviewed roadmap, design, testing guides, and the
  existing 2.3.1, 2.3.2, and 3.1.2 execplans.
- [x] (2026-04-13 00:00Z) Inspected the live code surfaces in `agent_backend`
  and `tool_registry` to confirm the missing translation seam.
- [x] (2026-04-13 00:00Z) Authored the initial ExecPlan draft.
- [ ] Stage A: lock the translation boundary and add failing tests.
- [ ] Stage B: add domain and port contracts for canonical-tool intake and
  backend translation.
- [ ] Stage C: implement runtime-adapter translation mapping for each backend.
- [ ] Stage D: wire translation into turn orchestration and canonical tool
  discovery.
- [ ] Stage E: add in-memory, PostgreSQL, and BDD coverage for happy and
  unhappy paths.
- [ ] Stage F: update design doc, user guide, and roadmap entry.
- [ ] Stage G: run full quality gates and capture evidence logs.

## Surprises & Discoveries

- `TurnExecutionRequest` is narrower than the design document implies. It
  carries only prompt text, conversation ID, and emitted tool calls, so 2.3.3
  needs a new internal runtime request type or a controlled extension of the
  existing one.

- `AgentRuntimePort` is the most natural place for backend-specific translation
  because it already owns backend-native session creation and turn execution.
  There is no separate backend adapter trait in code today.

- `ToolRouterPort` only routes calls. It is not currently the right place to
  obtain the canonical tool catalogue unless implementation later proves that a
  separate query port creates disproportionate churn.

- The repository already has a durable tool catalogue in `tool_registry`, so
  2.3.3 should not introduce a second persisted copy of translated schemas
  unless a concrete runtime requirement forces it.

## Decision Log

- Decision: prefer a local `agent_backend` query port for canonical tool
  discovery rather than widening `ToolRouterPort`. Rationale: listing available
  tools and routing tool calls are related, but they are still a query/command
  split. A local port keeps the orchestration dependency explicit and allows an
  adapter over the existing `tool_registry` catalogue without leaking that
  bounded context into `agent_backend` domain code. Date/Author: 2026-04-13 /
  plan author.

- Decision: keep translation mapping code-owned per backend adapter rather than
  persisting translated schemas. Rationale: the canonical MCP definitions are
  already durable; translation rules are implementation knowledge tied to a
  backend adapter version, not tenant-owned business data. Date/Author:
  2026-04-13 / plan author.

- Decision: keep canonical call routing unchanged and inject translation only
  into the "available tools for this turn" preparation path. Rationale: 3.1.2
  already owns call routing, validation, governance, and auditing. 2.3.3 should
  not fork that behaviour. Date/Author: 2026-04-13 / plan author.

- Decision: validate translation in two layers. First, translation code must
  reject unsupported canonical schema shapes with typed errors. Second, each
  runtime adapter must expose or embody an acceptance check that proves the
  translated payload is usable by that backend. Rationale: one layer preserves
  canonical semantics, and the other proves backend compatibility. Date/Author:
  2026-04-13 / plan author.

## Plan of work

### Stage A: establish the seam and write the failing tests first

Start by encoding the behaviour that 2.3.3 must deliver before changing the
service code.

Add or update tests in these areas:

- `src/agent_backend/tests/turn_orchestration_tests/` for focused unit tests.
- `tests/in_memory/agent_turn_orchestration_tests.rs` for service-level
  integration with memory adapters.
- `tests/postgres/agent_turn_orchestration_tests/` for orchestration with
  PostgreSQL-backed backend registry and session persistence.
- `tests/features/` plus a new BDD step module and scenario entrypoint for
  translated-tool behaviour.

The first red tests should prove these cases:

1. Happy path: a simple canonical MCP object schema translates for the
   "claude_code_sdk" backend and the turn executes successfully.
2. Happy path: the same canonical schema translates for the "codex_cli"
   backend and the turn executes successfully.
3. Unhappy path: an unsupported canonical schema shape fails before runtime
   execution begins.
4. Edge case: required-field preservation is enforced, so translation cannot
   silently drop a canonical required parameter.
5. Edge case: a backend receives the translated schema automatically; no test
   manually injects backend-specific tool schema payloads.

The BDD scenarios should remain service-visible. They should assert that the
turn succeeds or fails for typed reasons, not that an exact private JSON blob
matches line-for-line.

### Stage B: add the domain and port contracts

Implement the smallest set of contracts needed to move canonical tool
definitions through `agent_backend` without leaking backend-private payloads
into unrelated modules.

Planned edits:

- Add a new domain module under `src/agent_backend/domain/` for canonical tool
  definition intake and translated-tool envelopes. Split this into a dedicated
  file rather than enlarging `turn.rs`.
- Add a new `agent_backend` port that lists the canonical tools available to a
  turn. The port should return `agent_backend`-owned DTOs rather than raw
  `tool_registry` domain types.
- Extend `src/agent_backend/ports/runtime.rs` with backend-specific translation
  support. This should either be a new `translate_tool_schema` method or an
  equivalent batch-translation method if that keeps the runtime contract
  clearer.
- Add typed translation and validation errors that distinguish unsupported
  canonical schema shapes, backend incompatibility, and infrastructure failure.

Preferred shape:

- Keep `ExecuteAgentTurnRequest` unchanged.
- Introduce an internal prepared runtime request type that carries the
  translated available tools.
- Keep translated backend payloads opaque outside the runtime seam, but ensure
  each translated item still carries enough identity to correlate it back to
  the canonical tool name in tests and error messages.

### Stage C: implement backend-specific translation mapping

Implement the translation rules in runtime adapters, beginning with the
in-memory runtime used by the existing orchestration tests.

Planned edits:

- Update `src/agent_backend/adapters/memory/runtime.rs` so the adapter can
  translate canonical tool definitions into backend-specific tool schema
  payloads and reject unsupported shapes.
- Add backend-profile fixtures for at least the currently documented backends:
  `claude_code_sdk` and `codex_cli`.
- Keep the mapping explicit and code-owned. Do not rely on ad hoc string munging
  at call sites.

The in-memory runtime should be strict on purpose. It should reject
untranslated or malformed backend payloads so that tests prove the
orchestration path really performed translation.

If the implementation later introduces real backend adapters beyond the memory
runtime, apply the same contract tests to each one before wiring them into the
orchestrator.

### Stage D: wire translation into orchestration and tool discovery

Update the turn orchestration flow so translation becomes part of turn
preparation rather than an afterthought.

Planned edits:

- Introduce a small translation-focused service or helper under
  `src/agent_backend/services/` if keeping the logic inside
  `AgentTurnOrchestratorService` would make the orchestrator too large.
- In the orchestration flow, load canonical available tools before calling the
  runtime.
- Translate those tool definitions for the chosen backend.
- Validate the translation result and stop the turn early on failure.
- Pass the prepared translated tool surface into `execute_turn`.

For the canonical tool source, add an adapter that wraps the existing
`tool_registry` catalogue or discovery surface and maps it into the local
`agent_backend` DTOs. This preserves the hexagonal boundary:

- `agent_backend` owns the port it needs,
- `tool_registry` remains the canonical owner of discovered tool definitions,
- the adapter boundary performs the bounded-context mapping.

Do not alter tool-call routing itself. Tool invocations emitted by the runtime
must continue to flow through `ToolRouterPort` exactly as they do today.

### Stage E: complete the full test matrix

Once the green path exists, expand coverage until the success criteria are
provable.

Unit tests with `rstest` should cover:

- translation for each supported backend,
- unsupported schema rejection,
- required-field preservation,
- deterministic correlation between canonical tool name and translated tool
  payload,
- orchestration failure when canonical tool discovery fails, and
- orchestration failure when backend translation fails.

In-memory integration tests should cover:

- turn execution with discovered canonical tools,
- repeated use of the same canonical tool surface across multiple backends, and
- the absence of manual backend-specific tool schema injection in test setup.

PostgreSQL integration tests should cover:

- tool discovery through the durable catalogue plus turn execution with
  translation enabled,
- persisted backend registrations combined with translated-tool execution, and
- unhappy paths where canonical catalogue contents are incompatible with a
  backend translator.

BDD coverage with `rstest-bdd` should cover:

- successful automatic translation for a supported backend, and
- early failure for an unsupported canonical schema shape.

If translation does not require a new database migration, the PostgreSQL tests
must still exercise the existing persisted catalogue and session tables
together. If a migration does become necessary, bump the template database name
in `tests/postgres/helpers.rs` or the relevant helper module so the new schema
is applied in local environments.

### Stage F: update design and user documentation, then mark the roadmap item

After the code and tests are stable, update these documents:

- `docs/corbusier-design.md`
  Record the accepted translation boundary, dispatch rule, supported canonical
  schema subset, and whether translation remains code-owned rather than
  persisted.
- `docs/users-guide.md`
  Add or expand a user-facing section explaining that Corbusier automatically
  translates canonical MCP tool schemas for supported backends, and describe
  the failure mode for unsupported schema shapes.
- `docs/roadmap.md`
  Mark item `2.3.3` and its child bullets done only after all validation gates
  pass.

### Stage G: run the full validation gates and capture evidence

Use the repository-required `tee` pattern for every gate so failures are
inspectable after the command completes.

Run these commands:

```plaintext
set -o pipefail && OPENSSL_NO_VENDOR=1 make check-fmt 2>&1 | tee /tmp/2-3-3-check-fmt.log
set -o pipefail && OPENSSL_NO_VENDOR=1 make lint 2>&1 | tee /tmp/2-3-3-lint.log
set -o pipefail && OPENSSL_NO_VENDOR=1 make test TEST_FLAGS='--profile long --all-targets --all-features' 2>&1 | tee /tmp/2-3-3-test.log
set -o pipefail && PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/2-3-3-markdownlint.log
set -o pipefail && make nixie 2>&1 | tee /tmp/2-3-3-nixie.log
```

Before the PostgreSQL-backed tests, ensure the local toolchain can access the
embedded PostgreSQL helper described in
`docs/pg-embed-setup-unpriv-users-guide.md`. If the binary is missing, install
or prepare it before rerunning the test gate rather than mutating tests to
avoid PostgreSQL coverage.

Expected evidence at the end of Stage G:

- the new unit, integration, and BDD tests fail before the feature and pass
  after it,
- the full repo gates pass,
- the design doc and user guide reflect the shipped behaviour, and
- `docs/roadmap.md` marks `2.3.3` complete.

## Outcomes & Retrospective

This section is intentionally blank until implementation is approved and
completed. When the feature lands, replace this paragraph with a concise record
of what shipped, what changed from the draft, what evidence proved success, and
what follow-on work remains outside roadmap item `2.3.3`.
