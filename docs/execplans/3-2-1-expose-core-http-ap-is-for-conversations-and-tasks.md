# Expose core HTTP APIs for conversations and tasks (roadmap 4.2.1)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

Current roadmap numbering places this work at `4.2.1` in
[docs/roadmap.md](../roadmap.md). This file keeps the user-requested `3-2-1`
filename so the implementation task can be tracked without renaming the request.

Execution phase does not begin until this draft is explicitly approved.

## Purpose / big picture

Implement the first public HTTP API surface so external clients can create and
inspect conversations, manage tasks through the workflows Corbusier already
supports, and list or invoke tools through a stable, authenticated, versioned
interface.

After this change, a caller can:

1. Send authenticated HTTP requests to `/api/v1/...` routes for conversation,
   task, and tool workflows.
2. Receive a consistent versioned JSON envelope for both success and failure
   responses.
3. Rely on middleware to reject missing or invalid bearer tokens before domain
   services are invoked.

Observable success means:

1. Actix route tests prove the expected HTTP status codes, version metadata,
   and response shapes.
2. Behavioural tests prove happy and unhappy paths at the HTTP boundary,
   including missing authentication and invalid input.
3. Postgres-backed integration tests exercise the same API surface with
   `pg-embedded-setup-unpriv` enabled for local execution.

## Constraints

- Preserve hexagonal boundaries:
  - Business rules remain in existing bounded contexts under `src/message/`,
    `src/task/`, and `src/tool_registry/`.
  - The HTTP layer is a driving adapter only. Actix handlers, middleware, and
    DTO translation must live outside domain modules.
  - If a workflow is missing in the existing service layer, add or extend an
    application service in the owning bounded context rather than placing
    orchestration logic in handlers.
- Keep the initial scope to roadmap item `4.2.1`:
  - Include core conversation, task, and tool endpoints.
  - Include API versioning and authentication enforcement.
  - Do not implement SSE or WebSocket streaming from roadmap `4.3.1`.
  - Do not implement RBAC from roadmap `5.1.2`.
  - Do not implement admin endpoints.
- Reuse the existing Actix runtime in `src/main.rs` and the health adapter
  pattern already present in `src/health/`.
- Use `rstest` for unit and adapter tests, and `rstest-bdd` for behavioural
  scenarios where user-visible HTTP behaviour is expressed.
- Use `pg-embedded-setup-unpriv` fixtures and helpers for Postgres-backed
  integration coverage.
- Keep files below the repository's 400-line cap by splitting modules early.
- Maintain strict lint and formatting gates:
  - `make check-fmt`
  - `make lint`
  - `make test TEST_FLAGS='--profile long --all-targets --all-features'`
  - `make fmt`
  - `PATH=/root/.bun/bin:$PATH make markdownlint`
  - `make nixie`
- Record implementation decisions in `docs/corbusier-design.md`.
- Update `docs/users-guide.md` with the HTTP API behaviour a user or integrator
  must know.
- Mark roadmap item `4.2.1` done in `docs/roadmap.md` only after all feature
  and documentation gates pass.

## Tolerances (exception triggers)

- Scope: stop and escalate if the implementation exceeds 40 changed files or
  2,500 net lines.
- Security: stop and escalate if authentication cannot be enforced without
  simultaneously implementing a full identity store or refresh-token/session
  subsystem.
- Tenant isolation: stop and escalate if exposing Postgres-backed endpoints
  would rely on tenant isolation that the current schema and Row-Level Security
  (RLS) do not actually provide.
- Dependencies: stop and escalate if the work needs more than two new runtime
  crates beyond a focused JWT/auth solution.
- Contract ambiguity: stop and escalate if the required endpoint set cannot be
  reconciled between `docs/roadmap.md`, `docs/corbusier-design.md`, and the
  workflows currently implemented in `message`, `task`, and `tool_registry`.
- Iterations: stop and escalate if one failure class survives four focused
  fix-and-rerun cycles.

## Risks

- Risk: Corbusier currently has Actix only for health checks, not for the core
  API surface. Severity: medium. Likelihood: high. Mitigation: treat HTTP as a
  dedicated outer adapter module with its own state, error mapping, auth, and
  tests rather than accreting routes directly in `main.rs`.
- Risk: the `message` bounded context exposes message persistence but not a
  conversation-oriented application service. Severity: high. Likelihood: high.
  Mitigation: add a conversation service and any missing ports inside the
  `message` context before wiring HTTP routes.
- Risk: the `task` bounded context exposes lifecycle workflows, but read-side
  HTTP needs may exceed the current public service surface. Severity: medium.
  Likelihood: medium. Mitigation: add a focused query service or narrow,
  explicit read methods rather than bloating `TaskLifecycleService`.
- Risk: the current Postgres adapters for messages and tasks set
  `app.tenant_id`, but the repository comments and roadmap still defer real RLS
  enforcement to roadmap `2.5.3`. Severity: high. Likelihood: high. Mitigation:
  treat tenant-safe production exposure as a design checkpoint; if schema work
  must be widened, record that in the design doc and escalate.
- Risk: authentication scope can expand into roadmap `5.1.1` and `5.1.2`.
  Severity: high. Likelihood: medium. Mitigation: enforce one initial bearer
  token validation path for `/api/v1` and defer richer session lifecycle and
  role checks.

## Progress

- [x] (2026-03-27 00:00Z) Reviewed roadmap item `4.2.1`, the referenced design
  sections, repository instructions, and test/tooling guides.
- [x] (2026-03-27 00:00Z) Inspected current Actix entrypoint, existing bounded
  contexts, and test patterns to ground the plan in the current tree.
- [x] (2026-03-27 00:00Z) Authored the initial ExecPlan draft in this file.
- [ ] Await approval before execution begins.
- [ ] Stage A: lock the HTTP contract, versioning rule, and auth boundary.
- [ ] Stage B: add missing application services and ports in bounded contexts.
- [ ] Stage C: implement the HTTP adapter, middleware, and composition root.
- [ ] Stage D: add unit, behavioural, in-memory, and Postgres-backed tests.
- [ ] Stage E: update design and user docs, then mark roadmap item `4.2.1`
  done.
- [ ] Stage F: run full code and documentation quality gates and capture
  evidence.

## Surprises & Discoveries

- The current roadmap numbers this work as `4.2.1`, not `3.2.1`.
- `src/main.rs` already starts Actix Web, but it only wires the health-check
  adapter from `src/health/actix_adapter.rs`.
- `src/task/services/lifecycle.rs` already covers create-from-issue, branch
  association, pull-request association, and state transitions, so the HTTP API
  should expose those existing capabilities rather than inventing new task
  behaviours.
- `src/tool_registry/services/discovery/mod.rs` already exposes a usable read
  and call surface through `list_catalog()` and `call_tool()`.
- The `message` context has `ConversationId`, message persistence, and
  conversation table models, but no `ConversationRepository` or
  `ConversationService` exists yet.
- The Postgres adapters for `message` and `task` document that tenant context
  is prepared for RLS but not yet enforced by schema/policies, which is a real
  production exposure risk for authenticated HTTP endpoints.

## Decision Log

- Decision: implement the HTTP surface as a dedicated outer adapter module
  (for example `src/http_api/`), not as ad hoc route functions embedded in
  existing bounded contexts. Rationale: the HTTP layer is a delivery mechanism,
  not part of the core business model. Date/Author: 2026-03-27 / plan author.
- Decision: keep handlers thin and route missing behaviour back into the owning
  bounded context. Rationale: preserves the dependency rule and avoids Actix
  becoming a de facto application-service layer. Date/Author: 2026-03-27 / plan
  author.
- Decision: use URL path versioning (`/api/v1/...`) as the mandatory versioning
  mechanism for roadmap `4.2.1`, and include the version in response metadata.
  Rationale: this directly satisfies `docs/corbusier-design.md` §6.3.1.5 while
  keeping header negotiation optional and non-blocking. Date/Author: 2026-03-27
  / plan author.
- Decision: enforce authenticated access on all `/api/v1` conversation, task,
  and tool routes through bearer-token middleware, while leaving health routes
  unauthenticated. Rationale: matches the design tables and keeps auth concerns
  centralized. Date/Author: 2026-03-27 / plan author.
- Decision: scope the initial task API to workflows already implemented in the
  core: create from issue, fetch by id, transition state, associate branch, and
  associate pull request. Rationale: this exposes existing behaviour without
  expanding into unimplemented list/delete semantics. Date/Author: 2026-03-27 /
  plan author.

## Outcomes & Retrospective

Pending implementation.

The target outcome is a versioned, authenticated Actix API that exposes the
current conversation, task, and tool workflows without violating hexagonal
boundaries. This section must be updated during execution with the final
behaviour, validation evidence, and any lessons learned.

## Context and orientation

Relevant repository state before implementation:

- `src/main.rs` starts Actix and currently configures only `health_routes`.
- `src/health/actix_adapter.rs` shows the established pattern for an Actix
  driving adapter plus `actix_web::test` coverage.
- `src/task/services/lifecycle.rs` already exposes task lifecycle operations
  suitable for HTTP command endpoints.
- `src/tool_registry/services/discovery/mod.rs` already exposes tool catalogue
  listing and tool call routing suitable for HTTP tool endpoints.
- `src/message/ports/repository.rs` and its adapters provide append-only
  message persistence but no conversation-level service abstraction yet.
- Behaviour-driven tests currently live under `tests/features/` plus either a
  single step file (for simpler features) or `*_steps/` directories with a
  scenario runner.
- Postgres integration tests are aggregated through `tests/postgres.rs` and use
  the shared cluster/template helper in `tests/postgres/helpers.rs`.

Primary files and directories expected to change during implementation:

- New outer adapter module such as:
  - `src/http_api/mod.rs`
  - `src/http_api/auth.rs`
  - `src/http_api/error.rs`
  - `src/http_api/response.rs`
  - `src/http_api/state.rs`
  - `src/http_api/routes/{conversations,tasks,tools}.rs`
- `src/main.rs` for composition-root wiring.
- `src/message/` for new conversation-level ports/services and their tests.
- `src/task/` only if a narrow query service or public read method is required.
- `tests/features/http_api_surface.feature`.
- Either `tests/http_api_surface_steps.rs` or
  `tests/http_api_surface_steps/` plus a scenario runner.
- `tests/in_memory/http_api_surface_tests.rs`.
- `tests/postgres/http_api_surface_tests.rs`.
- `tests/postgres/helpers.rs` if new additive migrations become necessary.
- `docs/corbusier-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`.

Reference documents that govern the design and validation of this feature are
listed here so the implementation can proceed without rediscovery.[^1][^2][^3]
[^4][^5][^6][^7][^8]

[^1]: `docs/roadmap.md` item `4.2.1`.
[^2]: `docs/corbusier-design.md` §2.1.4, §6.2.1, §6.3.1.5, and §6.4.1.
[^3]: `docs/rust-testing-with-rstest-fixtures.md`.
[^4]: `docs/rstest-bdd-users-guide.md`.
[^5]: `docs/pg-embed-setup-unpriv-users-guide.md`.
[^6]: `docs/reliable-testing-in-rust-via-dependency-injection.md`.
[^7]: `docs/rust-doctest-dry-guide.md`.
[^8]: `docs/complexity-antipatterns-and-refactoring-strategies.md`.

## Target HTTP contract

The implementation should lock the following initial HTTP contract before code
is written:

- Conversation routes:
  - `POST /api/v1/conversations`
  - `GET /api/v1/conversations/{conversation_id}/history`
  - `POST /api/v1/conversations/{conversation_id}/messages`
- Task routes:
  - `POST /api/v1/tasks`
  - `GET /api/v1/tasks/{task_id}`
  - `PUT /api/v1/tasks/{task_id}/state`
  - `PUT /api/v1/tasks/{task_id}/branch`
  - `PUT /api/v1/tasks/{task_id}/pull-request`
- Tool routes:
  - `GET /api/v1/tools`
  - `POST /api/v1/tools/calls`

All of these routes must:

- require `Authorization: Bearer <token>`;
- reject missing or invalid tokens with `401 Unauthorized`;
- return a versioned JSON envelope containing response metadata including
  `version`, `request_id`, and `timestamp`;
- map domain and validation failures to stable HTTP error responses instead of
  exposing adapter-specific errors.

Streaming routes, admin routes, and list/delete enrichment stay out of scope
for this roadmap item.

## Plan of work

### Stage A: Lock the API contract and composition boundary

Before writing any route code, define the explicit HTTP boundary and the
minimal request or response contract for each endpoint.

Planned work:

- Add a design subsection to `docs/corbusier-design.md` during implementation
  that records:
  - the exact v1 endpoint set;
  - the response envelope shape;
  - the initial JWT claim contract used to build `RequestContext`;
  - anything intentionally deferred, such as streaming or RBAC.
- Define a small HTTP state/composition struct that owns only application
  services and infrastructure adapters already assembled at the composition
  root.
- Define one shared error-mapping layer and one shared response-envelope
  builder so handlers do not each reinvent formatting logic.

Go/no-go checkpoint:

- Do not proceed until the route list, auth rule, and versioning rule are
  written down and consistent with the design document.

### Stage B: Add missing application services in the owning bounded contexts

Fill only the business-layer gaps required for the HTTP adapter.

Planned work:

- In `src/message/`:
  - introduce a conversation-oriented port and service for creating a
    conversation and appending or retrieving history;
  - add a minimal conversation domain type if the service cannot stay coherent
    with `ConversationId` and existing message types alone;
  - keep persistence details behind ports and adapters.
- In `src/task/`:
  - expose a narrow read-side service for `GET /tasks/{id}` if reusing
    `TaskLifecycleService` would leak private helper behaviour into the HTTP
    adapter;
  - avoid adding list/filter endpoints unless the contract is widened by
    explicit approval.
- In `src/tool_registry/`:
  - reuse `ToolDiscoveryRoutingService::list_catalog()` and `call_tool()`
    rather than introducing HTTP-specific behaviour in the bounded context.

Required tests before green-phase implementation:

- `rstest` unit tests for any new message or task application services.
- Happy and unhappy paths for create/fetch/append/transition behaviours.
- Clock or environment dependencies must be injected, not read directly from
  globals.

Go/no-go checkpoint:

- Do not proceed to full HTTP wiring until missing service-layer behaviour is
  covered by failing tests and the new interfaces remain domain-owned.

### Stage C: Build the HTTP driving adapter

Implement the Actix-facing adapter as a pure delivery layer.

Planned work:

- Add route modules for conversations, tasks, and tools.
- Add request DTOs and response DTOs for each route family.
- Add shared middleware or extractors for:
  - bearer token validation;
  - request identifier generation;
  - `RequestContext` construction from JWT claims plus generated correlation
    identifiers.
- Add one response-envelope type similar to the design example:

  ```rust
  pub struct ApiResponse<T> {
      pub success: bool,
      pub data: Option<T>,
      pub error: Option<ApiError>,
      pub metadata: ResponseMetadata,
  }
  ```

- Update `src/main.rs` to wire the HTTP module alongside health routes without
  embedding business logic there.
- Keep authentication middleware scoped to `/api/v1`, not to `/health/*`.

Go/no-go checkpoint:

- Do not continue past this stage if handlers begin depending directly on
  Diesel pools or repository adapters rather than application services.

### Stage D: Add behaviour-first HTTP tests

Express the user-visible contract through tests before final hardening.

Planned tests:

- Adapter-level route tests with `actix_web::test` and `rstest`:
  - authenticated conversation creation succeeds;
  - missing bearer token returns `401`;
  - malformed token returns `401`;
  - unknown conversation or task identifiers return `404`;
  - invalid task transition returns the mapped client error;
  - tool call validation failure returns a structured error response;
  - every success and failure response includes `metadata.version == "v1"`.
- Behavioural tests with `rstest-bdd`:
  - create a conversation and append a message through HTTP;
  - create a task from issue metadata through HTTP;
  - transition a task state through HTTP;
  - list tools and invoke a tool through HTTP;
  - reject unauthenticated access.
- In-memory integration tests:
  - end-to-end route wiring with in-memory adapters to keep feedback fast.
- Postgres integration tests:
  - run the same route contract against Postgres-backed services using the
    existing embedded-cluster patterns;
  - if a new migration is added, bump `tests/postgres/helpers.rs::TEMPLATE_DB`
    so stale templates are not reused.

Go/no-go checkpoint:

- Do not declare the feature complete until the same HTTP contract passes with
  both in-memory and Postgres-backed wiring, or a documented tenant-isolation
  escalation has been raised.

### Stage E: Documentation, roadmap completion, and hardening

After the code path is complete:

- update `docs/corbusier-design.md` with the dated implementation decisions;
- update `docs/users-guide.md` with:
  - the endpoint list;
  - required authentication header;
  - example request and response envelopes;
  - versioning expectations;
- mark roadmap item `4.2.1` and its sub-bullets as done in
  `docs/roadmap.md`;
- rerun the full code and documentation quality gates.

Go/no-go checkpoint:

- The roadmap item is not complete until both the code gates and documentation
  gates pass cleanly.

## Concrete steps

Run all commands from repository root: `/home/user/project`.

1. Fast formatting and lint feedback while shaping the adapter:

   ```bash
   set -o pipefail; make check-fmt 2>&1 | tee /tmp/4-2-1-check-fmt.log
   set -o pipefail; make lint 2>&1 | tee /tmp/4-2-1-lint.log
   ```

   Expected signal: new HTTP modules compile cleanly and Clippy does not force
   broad suppressions.

2. Targeted test feedback while iterating on the new API surface:

   ```bash
   set -o pipefail; cargo nextest run --all-targets --all-features http_api_surface 2>&1 | tee /tmp/4-2-1-targeted-tests.log
   ```

   Expected signal: the new HTTP route and scenario suites fail first for the
   intended red-phase reasons, then pass as implementation lands.

3. Full repository gates before marking the roadmap item complete:

   ```bash
   set -o pipefail; make test TEST_FLAGS='--profile long --all-targets --all-features' 2>&1 | tee /tmp/4-2-1-test.log
   set -o pipefail; make fmt 2>&1 | tee /tmp/4-2-1-fmt.log
   set -o pipefail; PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/4-2-1-markdownlint.log
   set -o pipefail; make nixie 2>&1 | tee /tmp/4-2-1-nixie.log
   ```

   Expected signal: all commands exit `0`, with the test suite using the `long`
   nextest profile to avoid the default 5-minute global timeout.

## Validation and acceptance

Acceptance is behavioural:

1. Authenticated callers can create a conversation, append a message, and read
   conversation history through `/api/v1` routes.
2. Authenticated callers can create a task from issue metadata, fetch a task by
   id, transition task state, and associate branch or pull-request references.
3. Authenticated callers can list the tool catalogue and invoke a tool through
   the HTTP adapter.
4. Missing or invalid bearer tokens are rejected before domain services run.
5. Successful and failing responses both carry explicit API version metadata.

Quality criteria:

- New unit and adapter tests use `rstest`.
- Behavioural tests use `rstest-bdd` where the HTTP contract is better
  expressed as user-observable scenarios.
- Postgres-backed tests run locally with `pg-embedded-setup-unpriv`.
- `make check-fmt`, `make lint`, `make test TEST_FLAGS='--profile long
  --all-targets --all-features'`, `make fmt`, `PATH=/root/.bun/bin:$PATH make
  markdownlint`, and `make nixie` all pass.

## Idempotence and recovery

- The verification commands above are safe to rerun.
- Route and middleware tests should use isolated app instances so repeated runs
  do not share state.
- If a new migration is introduced and Postgres tests appear stale, bump the
  template name in `tests/postgres/helpers.rs` and rerun the suite.
- If documentation formatting introduces unrelated markdown churn, restore the
  unrelated files before finalizing the implementation diff.

## Interfaces and dependencies

Planned core interfaces and module responsibilities:

- HTTP adapter:
  - route handlers take validated DTOs plus injected application services;
  - middleware or extractors produce authenticated request context;
  - response helpers format the versioned API envelope.
- Message bounded context:
  - owns conversation creation, append-message, and history retrieval
    workflows.
- Task bounded context:
  - owns create, fetch, state-transition, branch-association, and
    pull-request-association workflows.
- Tool registry bounded context:
  - owns tool catalogue listing and tool-call execution.

Initial auth expectation for v1:

- bearer JWT validation only;
- claims must be sufficient to construct `RequestContext`, at minimum:
  `tenant_id`, `user_id`, `session_id`, and expiry information;
- request correlation id is generated server-side when not supplied through a
  trusted header.

If the implementation needs broader auth semantics than this, trigger
escalation instead of silently widening roadmap scope.

## Artifacts and notes

Implementation must capture evidence in this document as work proceeds:

- which endpoint contract shipped;
- which auth claim set was chosen;
- whether tenant-isolation concerns required roadmap escalation;
- the final pass/fail summaries for the quality gates listed above.

## Revision note

- 2026-03-27: Initial draft created from roadmap item `4.2.1`, the HTTP API,
  versioning, and authentication sections of `docs/corbusier-design.md`, the
  local Actix wiring, and the repository's established test and documentation
  conventions.
