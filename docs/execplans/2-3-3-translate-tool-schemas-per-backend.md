# Translate tool schemas per backend (roadmap 2.3.3)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

This plan covers roadmap item `2.3.3` in `docs/roadmap.md`.

Execution phase does not begin until this draft is explicitly approved.

## Purpose / big picture

Roadmap item `2.3.2` established the core turn-orchestration path and
session-continuity rules, while roadmap items `3.1.1` and `3.1.2` established
canonical Model Context Protocol (MCP) server lifecycle, tool discovery,
cataloguing, and call routing. Corbusier can now discover canonical tool
definitions and route canonical tool calls, but it still lacks the missing
bridge between those canonical MCP definitions and the backend-specific tool
schema formats expected by individual agent runtimes.

After this change, Corbusier will:

1. Read canonical tool definitions from the tool registry during turn
   orchestration.
2. Translate each canonical MCP tool schema into the schema shape required by
   the selected backend profile.
3. Validate that the translated schemas preserve the canonical MCP contract and
   are accepted by the target backend adapter without manual edits.
4. Supply backend-ready tool definitions to the runtime automatically, so the
   same registered tools can be presented consistently across Claude Code SDK,
   Codex CLI App Server, and future backend profiles.

Observable success means:

1. Unit tests (`rstest`) cover translation mapping, validation, and failure
   paths for each supported backend profile.
2. Behaviour tests (`rstest-bdd`) prove that a registered backend receives
   usable tool definitions automatically from the orchestration path.
3. In-memory and PostgreSQL-backed integration tests prove the end-to-end flow
   from discovered canonical MCP tools to backend-accepted translated schemas.
4. `docs/corbusier-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`
   are updated only after all quality gates pass.

## Constraints

- Preserve strict hexagonal boundaries.
  Domain translation and validation logic must remain in pure `agent_backend`
  code; ports define contracts; adapters provide backend- or storage-specific
  behaviour; adapters must not call other adapters directly.
- Treat the tool registry as the canonical source of truth for MCP tool
  definitions. Translation must consume canonical definitions through a port or
  port-owned data transfer object (DTO), not through direct access to
  `tool_registry` adapter internals.
- Keep scope to roadmap `2.3.3`.
  Do not backfill unrelated message-history loading, workspace management, hook
  execution, or file-editor enforcement unless they are required to make schema
  translation reachable from the current turn path.
- Preserve all delivered behaviour for roadmap items `2.3.1`, `2.3.2`,
  `3.1.1`, and `3.1.2`.
- Keep every Rust source file below the repository's 400-line limit by
  extracting small modules early.
- Add module-level `//!` comments and public Rustdoc `///` comments for all new
  public modules and APIs.
- Use `rstest` for unit and integration fixtures, and `rstest-bdd` for
  behavioural scenarios where user-visible behaviour is expressed.
- Use `pg-embedded-setup-unpriv` fixtures for PostgreSQL-backed integration
  coverage.
- Keep comments and documentation in en-GB-oxendict spelling.
- Run repository gates through Makefile targets.
  For implementation turns this means `make check-fmt`, `make lint`, and
  `make test`; because this roadmap item also requires documentation updates,
  it also means `make fmt`, `make markdownlint`, and `make nixie`.
- For long-running validation commands, use `set -o pipefail` and `tee` to log
  output for later inspection, as required by `AGENTS.md`.
- Record implementation decisions in `docs/corbusier-design.md`.
- Update `docs/users-guide.md` for any server behaviour or operator-visible
  backend/tool changes introduced by this feature.
- Mark roadmap item `2.3.3` done in `docs/roadmap.md` only after all quality
  gates pass.

## Tolerances (exception triggers)

- Scope: stop and escalate if implementation exceeds roughly 35 changed files
  or 2,500 net lines.
- Interface: stop and escalate if the feature requires incompatible public API
  changes outside `agent_backend`, `tool_registry`, `test_support`, and their
  direct tests.
- Dependency: stop and escalate if a new external crate is needed for schema
  normalization or validation. The first implementation should prefer
  `serde_json` plus repository-local validation logic.
- Persistence: stop and escalate if delivering `2.3.3` would require a new
  persistent schema-mapping table rather than deriving translation profiles
  from existing backend registration metadata.
- Adapters: stop and escalate if backend-specific acceptance cannot be modeled
  with contract-test fakes until concrete backend adapters land.
- Iterations: stop and escalate if one failure class remains unresolved after
  4 focused fix-and-rerun cycles.
- Ambiguity: stop and escalate if the design document is insufficient to choose
  one backend schema shape for Claude Code SDK or Codex CLI App Server without
  inventing product behaviour.

## Risks

- Risk: current orchestration code does not yet fetch available tools or pass
  them to the runtime, even though `docs/corbusier-design.md` models that flow.
  Severity: high. Likelihood: high. Mitigation: make the contract change an
  explicit first-class stage in this plan and treat it as part of `2.3.3`,
  because schema translation is otherwise unreachable.

- Risk: no concrete Claude Code SDK or Codex CLI runtime adapters exist yet,
  so backend acceptance rules are partly implicit. Severity: high. Likelihood:
  medium. Mitigation: define explicit backend translation profiles and
  validator fakes keyed by registered backend identity, then record the
  assumptions in `docs/corbusier-design.md`.

- Risk: `agent_backend` could grow an improper compile-time dependency on
  `tool_registry` adapters or persistence details. Severity: medium.
  Likelihood: medium. Mitigation: introduce a read-only available-tools port
  and keep canonical tool definition mapping at the port boundary.

- Risk: backend-specific translation can silently drop required MCP schema
  information. Severity: high. Likelihood: medium. Mitigation: add invariant
  validation that checks tool name, description, required fields, and relevant
  schema keywords before the runtime receives translated definitions.

- Risk: behavioural tests may become brittle if they assert internal JSON shape
  instead of observable runtime acceptance. Severity: medium. Likelihood:
  medium. Mitigation: keep `rstest-bdd` scenarios focused on backend acceptance
  and orchestration outcomes, and leave fine-grained schema assertions to
  `rstest` unit tests.

- Risk: if a database migration becomes necessary, stale PostgreSQL template
  databases may mask missing schema updates. Severity: medium. Likelihood: low.
  Mitigation: avoid a migration unless it is genuinely required; if one is
  added, update `tests/postgres/migrations.rs` and bump
  `tests/postgres/helpers.rs` `TEMPLATE_DB`.

## Progress

- [x] (2026-04-10 00:00Z) Reviewed roadmap item `2.3.3` and the governing
  sections of `docs/corbusier-design.md`.
- [x] (2026-04-10 00:00Z) Reviewed testing and documentation guidance in the
  repository docs and `AGENTS.md`.
- [x] (2026-04-10 00:00Z) Inspected the current `agent_backend`,
  `tool_registry`, `test_support`, and existing ExecPlan patterns to draft this
  plan.
- [x] (2026-04-10 00:00Z) Authored the initial ExecPlan draft in this file.
- [ ] Await explicit approval before implementation.
- [ ] Execute stages A-G and keep this section current during implementation.

## Surprises & Discoveries

- The `execplans` skill referenced by older repository plans is not available
  in this session, so this plan was drafted manually while preserving the
  repository's existing ExecPlan structure.
- No Qdrant note-store MCP resources or templates are exposed in this session,
  so historical project memory could not be recalled through the documented
  protocol.
- The current `AgentTurnOrchestratorService` routes emitted tool calls through
  `ToolRouterPort`, but it does not yet load available tools before runtime
  execution.
- `TurnExecutionRequest` currently carries only `conversation_id`, `prompt`,
  and pre-scripted `tool_calls`; it does not yet carry backend-ready available
  tool definitions.
- Existing backend registration metadata already includes `name`, `provider`,
  and JSONB-backed capability fields, which looks sufficient to derive a
  backend translation profile without adding persistence.

## Decision Log

- Decision: treat the "available tools into runtime request" seam as part of
  roadmap `2.3.3`, not a separate unplanned refactor. Rationale: schema
  translation has no effect unless translated tool schemas are actually
  supplied to the backend runtime. Date/Author: 2026-04-10 / plan author.

- Decision: implement translation as an `agent_backend` concern with pure
  translation and validation logic behind explicit ports, rather than embedding
  backend-specific JSON rewriting inside runtime adapters. Rationale: preserves
  the hexagonal boundary and keeps translation logic testable without
  infrastructure. Date/Author: 2026-04-10 / plan author.

- Decision: consume canonical MCP tool definitions through a read-only port
  owned by `agent_backend`, backed by the tool registry. Rationale: keeps
  `tool_registry` as source of truth while preventing adapter-to-adapter
  coupling. Date/Author: 2026-04-10 / plan author.

- Decision: derive translation profile selection from existing backend
  registration metadata first, using backend name and provider as the stable
  selectors unless implementation evidence proves that a richer persisted
  profile is needed. Rationale: avoids premature schema changes and keeps
  profile selection explicit. Date/Author: 2026-04-10 / plan author.

- Decision: validate translations in two layers.
  First, verify structural invariants against the canonical MCP definition.
  Second, verify backend acceptance with adapter-contract tests that emulate
  each backend's expected schema shape. Rationale: both requirements are
  explicit in `docs/corbusier-design.md` sections `2.2.3` and `2.2.4`.
  Date/Author: 2026-04-10 / plan author.

## Outcomes & Retrospective

Initial planning outcome: the repository now has an execution-ready plan for
roadmap `2.3.3`, including the missing contract seam between discovered
canonical tools and backend runtime requests. Implementation outcomes, test
evidence, and lessons learned will be captured here after execution.

## Context and orientation

Relevant current repository state:

- `src/agent_backend/services/orchestrator/mod.rs` currently:
  resolves the backend, arbitrates sessions, executes the runtime turn, and
  routes emitted tool calls; it does not yet acquire available tools ahead of
  runtime execution.
- `src/agent_backend/ports/runtime.rs` currently defines only session creation,
  teardown, and `execute_turn` over a canonical `TurnExecutionRequest`.
- `src/agent_backend/domain/turn.rs` currently models tool-call requests,
  tool-call results, and runtime turn input and output, but not
  backend-specific available-tool schemas.
- `src/agent_backend/ports/tool_router.rs` currently exposes only
  `route_tool_call`; the design document's `list_available_tools` seam is not
  yet present in production code.
- `src/tool_registry/domain/tool.rs` defines canonical `McpToolDefinition`,
  which already carries tool name, description, input schema, and optional
  output schema.
- `src/tool_registry/services/discovery/mod.rs` and its adapters already prove
  canonical discovery, validation, routing, and audit behaviour for MCP tools.
- `src/test_support.rs` provides the current in-memory agent-turn stack and is
  the natural place to extend fixtures for backend-acceptance assertions.
- Existing behavioural coverage lives in:
  `tests/features/agent_turn_orchestration.feature` and
  `tests/features/tool_discovery_routing.feature`.

Likely files to change during implementation:

- `src/agent_backend/domain/`
- `src/agent_backend/ports/`
- `src/agent_backend/services/orchestrator/`
- `src/agent_backend/adapters/memory/`
- `src/tool_registry/` read-side adapter glue only, if a new read port needs a
  backing implementation
- `src/test_support.rs`
- `src/agent_backend/tests/`
- `tests/in_memory/`
- `tests/postgres/`
- `tests/features/`
- `tests/*_steps/`
- `docs/corbusier-design.md`
- `docs/users-guide.md`
- `docs/roadmap.md`

## Plan of work

### Stage A: lock the contracts and backend translation model

Define the missing seam between canonical discovered tools and backend runtime
execution before implementing behaviour:

- Decide whether the orchestrator should depend on a new `AvailableToolsPort`
  or whether `ToolRouterPort` should be expanded with a read path. Prefer the
  smaller, read-only port if it keeps responsibilities clearer.
- Introduce agent-backend-owned types for:
  canonical tool input to translation, backend-specific translated tool schema,
  translation profile identity, and typed translation-validation failures.
- Extend the runtime turn contract so the runtime can receive translated
  available tools without depending on `tool_registry` internals.
- Define backend profiles for the first supported backends:
  Claude Code SDK direct MCP mapping, Codex CLI App Server command-bridge
  mapping, and an unsupported/fallback path for unknown profiles.

Go/no-go: do not proceed until the contract shape is compile-clean, the
ownership boundary is explicit, and the design assumptions are written down in
the plan.

### Stage B: add red tests first

Describe the required behaviour before final implementation:

- Unit tests (`rstest`) for translation mapping:
  canonical required fields preserved, stable tool names and descriptions
  preserved, backend profile-specific shape produced, and unsupported profiles
  rejected with typed errors.
- Unit tests (`rstest`) for validation:
  translated schema round-trips against canonical expectations, lossy
  translations are rejected, invalid canonical schemas are rejected before
  runtime dispatch, and backend validator fakes reject malformed translated
  output.
- Behaviour tests (`rstest-bdd`) for observable orchestration:
  a backend receives translated available tools automatically, different
  registered backends receive different accepted schema shapes, and unsupported
  backend profiles fail before runtime execution begins.
- In-memory integration tests for the orchestrator path:
  discovered tools are translated and attached to the runtime request, runtime
  execution still routes emitted tool calls deterministically, and no manual
  per-backend edits are required in the test setup.
- PostgreSQL-backed integration tests:
  same behaviour as the in-memory path, using `pg-embedded-setup-unpriv`
  fixtures and the persisted tool catalogue.

Go/no-go: proceed only once the new tests fail for the expected missing-feature
reasons.

### Stage C: implement pure translation and validation modules

Build the core logic in small, infrastructure-free pieces:

- Add translation-profile selection logic keyed from backend registration
  metadata.
- Implement per-profile mapping from canonical MCP tool definitions to
  backend-specific translated tool schemas.
- Implement validation helpers that compare canonical and translated schemas
  for required invariants.
- Keep translation and validation separate so failures are attributable and
  test output remains specific.

Go/no-go: translation modules should be unit-tested green before orchestration
wiring begins.

### Stage D: wire orchestration to discovered tools and translated schemas

Make the current turn path actually use the new feature:

- Add the available-tools read dependency to `AgentTurnOrchestratorService`.
- Load canonical tool definitions before runtime execution.
- Translate and validate the tool schemas for the selected backend.
- Attach translated tool definitions to the runtime request.
- Fail the turn with typed orchestration errors before runtime execution if
  discovery, translation, or validation fails.

Go/no-go: the runtime must now receive translated available-tool definitions in
both tests and production wiring, or the stage is incomplete.

### Stage E: implement adapter and fixture support

Update supporting adapters without collapsing the boundary:

- Extend `InMemoryAgentRuntime` so tests can inspect the translated available
  tool payloads it received.
- Add read-side in-memory adapter support for the available-tools port.
- Add PostgreSQL-backed adapter glue that reads the tool catalogue through the
  existing tool-registry persistence model rather than inventing a duplicate
  store.
- Update `src/test_support.rs` and relevant fixtures so unit, behavioural, and
  integration tests can reuse one coherent setup path.
- If a migration is unavoidable, add it here and update the PostgreSQL test
  template versioning in the same stage.

Go/no-go: adapter support is complete only when the same high-level test data
can drive both in-memory and PostgreSQL acceptance paths.

### Stage F: finish behavioural and regression coverage

Complete the full test matrix and verify edge cases:

- Happy paths:
  Claude profile accepted, Codex profile accepted, multiple discovered tools
  translated in deterministic order.
- Unhappy paths:
  unknown backend profile, malformed canonical schema, translation drops a
  required field, backend validator rejects the translated payload, tool
  discovery lookup fails.
- Edge cases:
  optional output schema present, empty tool list, mixed required and optional
  fields, backend with `supports_tool_calls = false` if policy requires early
  rejection.

Go/no-go: unit, behavioural, in-memory, and PostgreSQL tests must all be green
before documentation updates and roadmap completion are allowed.

### Stage G: documentation, roadmap, and final gates

Close out the feature only after evidence is in place:

- Record the implemented `2.3.3` decisions in `docs/corbusier-design.md`,
  including: ownership of translation logic, supported backend profiles,
  validation rules, and any deliberate limitations.
- Update `docs/users-guide.md` with the user-visible behaviour:
  registered backends receive tool schemas automatically from the canonical MCP
  registry, plus any relevant failure surfaces or profile caveats.
- Mark roadmap item `2.3.3` and its sub-bullets done in `docs/roadmap.md`.
- Run final gates with logging:
  `make fmt`, `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  and `make nixie`, each via `set -o pipefail` and `tee`.

Done means: tool schemas are accepted by each registered backend profile
without manual edits, all tests pass, documentation is current, and the roadmap
is marked complete.
