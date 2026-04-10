# Stabilize the slice transport contract and development auth seam (roadmap 4.4.2)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

Current roadmap numbering places this work at `4.4.2` in
[docs/roadmap.md](../roadmap.md).

Execution does not begin until this plan is explicitly approved.

## Purpose / big picture

Implement roadmap item `4.4.2` so the first frontend vertical slice can rely on
a stable backend transport contract and a repeatable local browser auth seam
before `4.4.3` swaps the PWA from fixtures to live task create, detail, and
transition calls.

After this change, Corbusier should have:

1. Stable task create, detail, and transition HTTP contracts with golden
   response fixtures for happy and unhappy paths.
2. Contract-tested error responses aligned with the shared phase 4
   `actix-v2a` core HTTP primitives wherever those primitives touch the slice.
3. A development-only browser auth path that works for local preview and
   browser tests without settling the final production auth model.
4. A frontend transport seam that can consume the stabilized contract without
   collapsing the fixture-first route shell or breaking the frontend hexagon.

Observable success means:

1. `POST /api/v1/tasks`, `GET /api/v1/tasks/{task_id}`, and
   `PUT /api/v1/tasks/{task_id}/state` are covered by golden JSON fixtures and
   explicit contract tests.
2. Error responses for validation, auth, not-found, and conflict cases are
   validated in both in-memory and PostgreSQL-backed tests using `rstest` and
   `rstest-bdd` where behaviour is clearer at the HTTP boundary.
3. A contributor can run the frontend locally against the current API through a
   documented, development-only auth seam.
4. `4.4.3` can focus on wiring the live browser path rather than renegotiating
   transport shape, auth plumbing, or error handling.

## Constraints

- Preserve hexagonal boundaries on both sides of the seam:
  - task domain and service rules remain in `src/task/`;
  - HTTP route handlers, auth extractors, idempotency plumbing, and DTO
    mapping remain in `src/http_api/`;
  - frontend task domain, ports, and view-model mapping remain in
    `frontend-pwa/src/task_slice/domain` and
    `frontend-pwa/src/task_slice/ports`;
  - transport-specific fetch, auth-header, and envelope parsing logic stays in
    `frontend-pwa/src/task_slice/adapters/http`.
- Keep the scope to roadmap item `4.4.2`:
  - stabilize transport for task create, detail, and transition only;
  - adopt shared `error`, `idempotency`, and `openapi` primitives where they
    materially affect the slice;
  - document and implement a development-only auth seam for preview;
  - do not implement SSE transport or replay;
  - do not implement live branch or pull-request association UI;
  - do not widen into final production browser auth or session policy.
- Treat the phase 4 `actix-v2a` core HTTP contract dependency as real:
  - if the shared primitives are already consumable, integrate them directly;
  - if they are only partially available, adapt behind Corbusier-owned seams
    that can be replaced without changing the task endpoint contract;
  - do not invent a second incompatible envelope or idempotency scheme.
- Respect RFC 0002 ordering:
  - `4.4.1` remains the fixture-first shell;
  - `4.4.2` settles contract and auth seam;
  - `4.4.3` performs the live browser-path swap.
- Follow repository quality and documentation rules:
  - unit and integration tests use `rstest`;
  - behavioural tests use `rstest-bdd` where user-visible HTTP behaviour is
    clearer than low-level adapter assertions;
  - PostgreSQL-backed tests must use the `pg-embedded-setup-unpriv` fixtures;
  - update `docs/corbusier-design.md` with the transport/auth design decisions;
  - update `docs/users-guide.md` with the repeatable local preview path;
  - mark roadmap item `4.4.2` done in `docs/roadmap.md` only after all feature
    and documentation gates pass.

## Tolerances (exception triggers)

- Dependency: stop and escalate if the required phase 4 `actix-v2a` core HTTP
  primitives are unavailable and cannot be wrapped behind a narrow temporary
  compatibility seam.
- Scope: stop and escalate if stabilizing the slice contract requires list
  pagination, SSE, branch or pull-request mutation UI, or broader auth/session
  work beyond the task create/detail/transition path.
- Auth: stop and escalate if the development preview seam would require adding
  a production-grade login flow, cookie/session issuance, or persistent browser
  credential storage.
- Churn: stop and escalate if the work exceeds roughly 35 changed files or
  2,500 net lines before `4.4.3` begins.
- Frontend coupling: stop and escalate if the route modules would need to
  absorb raw HTTP envelope parsing or auth-token handling directly.
- Idempotency: stop and escalate if adopting shared idempotency requires new
  durable persistence machinery that belongs in a later API-wide contract step
  rather than in this slice-focused milestone.

## Risks

- Risk: the current API already ships a Corbusier-local success and error
  envelope, while the roadmap points to shared `actix-v2a` primitives that may
  not yet be present in-tree. Severity: high. Likelihood: high. Mitigation:
  stage the work around a contract inventory first, then adapt shared
  primitives behind narrow response/idempotency seams rather than rewriting
  handlers ad hoc.
- Risk: the current HTTP auth model is bearer JWT only, while
  `docs/corbusier-api-design.md` still treats cookie-based auth as the likely
  long-term default for SSE compatibility. Severity: medium. Likelihood: high.
  Mitigation: keep the browser auth seam explicitly development-only and prefer
  a same-origin dev proxy or server-side token injection over a
  production-looking browser login.
- Risk: `frontend-pwa/` currently boots on the fixture gateway only, so a naive
  live transport implementation could force `4.4.3` work into this milestone.
  Severity: high. Likelihood: medium. Mitigation: add the HTTP adapter seam and
  gateway selection path without switching the shipped route shell off fixtures
  by default.
- Risk: idempotency scope is underspecified for task transitions versus task
  creation. Severity: medium. Likelihood: medium. Mitigation: make the decision
  explicit during Stage A, document it in the design doc, and keep the task
  endpoint fixtures stable whichever path is chosen.
- Risk: contract tests can become brittle if they assert incidental metadata
  ordering or implementation-specific timestamps rather than stable payload
  fields. Severity: medium. Likelihood: medium. Mitigation: normalize dynamic
  fields and store golden fixtures only for stable contract surfaces.

## Progress

- [x] (2026-04-10 00:00Z) Reviewed roadmap item `4.4.2`, RFC 0002 sections
  `5.3` through `5.4`, the HTTP API design contract guidance, and the existing
  `4.4.1` and `4.2.1` execplans.
- [x] (2026-04-10 00:00Z) Inspected the current repository state for the task
  HTTP routes, auth extractor, shared response envelope, frontend fixture
  gateway, and stub HTTP gateway.
- [x] (2026-04-10 00:00Z) Authored the initial ExecPlan draft in this file.
- [ ] Await explicit user approval before implementation begins.

## Surprises & Discoveries

- `frontend-pwa/` already has the correct structural seam for this milestone:
  the route shell depends on `TaskSliceGateway`, and the live HTTP adapter is
  still a stub in
  `frontend-pwa/src/task_slice/adapters/http/http-task-gateway.ts`.
- The backend already exposes all task endpoints the slice needs:
  `POST /api/v1/tasks`, `GET /api/v1/tasks/{task_id}`, and
  `PUT /api/v1/tasks/{task_id}/state` are implemented in
  `src/http_api/routes/tasks.rs`.
- The current API response format is Corbusier-local:
  success responses use `{ success, data, error, metadata }`, while error
  payloads currently only guarantee `code` and `message`. The API design
  document and RFC 0002 expect convergence toward a shared error contract with
  `traceId` and optional `details`.
- The current auth surface is bearer-token only through
  `src/http_api/auth.rs`; there is no browser-facing development seam yet.
- The shared phase 4 `actix-v2a` dependency is referenced in docs and roadmap
  text, but no in-repo crate or module currently exposes those primitives. This
  milestone therefore needs an explicit compatibility strategy.

## Decision Log

- Decision: treat this milestone as a contract-hardening and seam-preparation
  step, not as the point where the default PWA route shell becomes live.
  Rationale: this preserves RFC 0002 ordering and keeps `4.4.3` responsible for
  the visible browser-path switch. Date/Author: 2026-04-10 / plan author.
- Decision: prefer a development-only same-origin proxy seam that injects or
  forwards bearer auth outside the browser runtime, while keeping production
  auth unresolved. Rationale: this gives a repeatable local path without
  pretending the bearer-token preview mechanism is the final browser model.
  Date/Author: 2026-04-10 / plan author.
- Decision: stabilize create, detail, and transition contracts together,
  including unhappy-path fixtures. Rationale: `4.4.3` depends on all three
  operations for the first live loop, and RFC 0002 calls out stable fixtures
  and contract-tested errors as the success criteria for `4.4.2`. Date/Author:
  2026-04-10 / plan author.

## Outcomes & Retrospective

Planning outcome: the execution order, quality gates, and seam boundaries for
`4.4.2` are now captured in one document. Implementation outcomes, deviations,
and lessons learned must be added after execution.

## Context and orientation

Current repository state relevant to `4.4.2`:

- `src/http_api/routes/tasks.rs` currently serves the slice-facing task
  endpoints and converts domain values into a `TaskResponse { task: TaskDto }`
  wrapper.
- `src/http_api/response.rs` currently provides Corbusier-local response
  envelopes and metadata, while `src/http_api/error/mod.rs` maps typed errors
  into that envelope.
- `src/http_api/auth.rs` currently enforces `Authorization: Bearer <jwt>` with
  HS256 claims carrying `sub`, `tenant_id`, `session_id`, and `exp`.
- `frontend-pwa/src/main.tsx` still boots with `createFixtureTaskGateway()`;
  the HTTP gateway exists only as a stub.
- `frontend-pwa/src/task_slice/ports/task-slice-gateway.ts` only models create
  and detail operations today, which is enough for `4.4.1` but not for the
  eventual live create → detail → transition loop in `4.4.3`.
- Existing backend contract tests cover task creation and transition happy
  paths, but they do not yet preserve golden fixtures or assert the richer
  shared error contract expected by this milestone.

Planned steady state after `4.4.2`:

- task create, detail, and transition endpoints have stable contract fixtures;
- error payloads are explicitly shaped and tested;
- idempotency handling is settled for the slice-facing mutations or clearly
  documented where the shared dependency is still pending;
- frontend preview has a repeatable development auth seam;
- the frontend HTTP adapter can be exercised in tests without forcing the app
  shell to abandon fixture-first default boot.

## Plan of work

### Stage A: inventory the contract and lock the scope boundary

Establish exactly what must stabilize and what stays deferred:

- Inventory the current task create, detail, and transition request and
  response shapes, including auth failures, validation failures, not-found, and
  conflict cases.
- Compare the live implementation with:
  - `docs/corbusier-api-design.md` section `HTTP API surface, pagination, SSE,
    and error contracts`;
  - RFC 0002 sections `5.3` and `5.4`;
  - the phase 4 `actix-v2a` dependency expectations.
- Decide and document the compatibility strategy for shared primitives:
  - direct reuse if the dependency is consumable;
  - a Corbusier-owned compatibility seam if not.
- Decide and document the idempotency boundary for this step:
  - minimum acceptable outcome is stable create/detail/transition contracts;
  - preferred outcome is shared idempotency handling on task creation and, if
    the shared primitive is ready without widening scope, task transition too.
- Freeze what remains out of scope:
  - SSE;
  - pagination;
  - branch and pull-request mutation transport;
  - final browser auth/session policy.

Go/no-go: do not proceed until the contract delta and shared-primitive
integration strategy are explicit in this plan and in the implementation notes.

### Stage B: stabilize the HTTP transport contract for the slice

Harden the server-side HTTP surface without moving business logic out of the
task bounded context:

- Refactor or extend the HTTP adapter so task endpoints share one explicit
  contract module for slice-facing DTOs, error schema usage, and any shared
  OpenAPI fragment wiring touched by this milestone.
- Align error responses with the shared envelope expectations where the slice
  cares:
  - stable `code`;
  - stable human-readable `message`;
  - stable trace or request identifier field;
  - structured `details` where validation or contract errors need machine
    parsing.
- Add or adapt idempotency handling on the slice-facing mutation endpoints in
  line with the Stage A decision.
- Add a fixture-generation or fixture-assertion harness for:
  - successful task creation;
  - successful task detail retrieval;
  - successful task transition;
  - validation failure;
  - unauthenticated request;
  - task not found;
  - invalid transition or other conflict case.
- Ensure the task handlers remain thin and continue delegating workflow logic
  to `TaskLifecycleService`.

Go/no-go: do not proceed until the task HTTP contract can be asserted without
depending on browser code or fixture adapters.

### Stage C: implement the development auth seam and frontend transport seam

Add the repeatable local preview path without freezing production auth:

- Choose one development-only preview seam and document it as temporary:
  - preferred: same-origin Vite proxy that injects or forwards a configured
    bearer token to the backend;
  - fallback: development-only bearer-token injection configured entirely in
    the frontend dev environment.
- Keep the backend auth boundary narrow:
  - continue using the current authenticated task endpoints;
  - do not add a production login flow;
  - only add preview-only helper configuration if the seam cannot be achieved
    via dev proxying alone.
- Extend the frontend transport seam:
  - add a real HTTP task gateway aligned to the stabilized task contract;
  - add shared transport parsing for success and error envelopes;
  - add a gateway-selection mechanism so fixture mode remains available while
    live transport can be exercised in targeted tests and later enabled by
    `4.4.3`.
- Expand the task-slice port only as far as needed to match the stabilized
  slice contract and unblock `4.4.3`, while keeping route modules transport
  agnostic.

Go/no-go: do not proceed until a contributor can run a documented local preview
path that reaches the current API with repeatable auth behaviour.

### Stage D: prove the contract with unit, behavioural, and PostgreSQL tests

Validate the seam at the right layers:

- Rust unit and adapter tests with `rstest`:
  - error mapping and envelope serialization;
  - auth extractor edge cases;
  - idempotency behaviour if adopted in this milestone;
  - golden fixture normalization and comparison helpers.
- Rust behavioural tests with `rstest-bdd`:
  - authenticated create, detail, and transition scenarios;
  - unauthenticated and invalid-input scenarios;
  - conflict or not-found scenarios visible at the HTTP boundary.
- PostgreSQL-backed tests using `pg-embedded-setup-unpriv`:
  - prove the same contract against the database-backed task workflow;
  - ensure fixture expectations hold under the persistent adapter.
- Frontend adapter tests:
  - success-envelope parsing;
  - error-envelope parsing and mapping to `TaskGatewayError`;
  - auth seam configuration behaviour;
  - no route-level transport leakage.

Go/no-go: proceed to documentation only when contract tests pass in both
in-memory and PostgreSQL-backed modes and the frontend transport seam is
covered by focused tests.

### Stage E: document the decisions and close the roadmap item

- Update `docs/corbusier-design.md` with:
  - the stabilized slice contract boundary;
  - the adopted compatibility strategy for shared `actix-v2a` primitives;
  - the development auth seam and its explicitly temporary status;
  - any idempotency decision or deferral taken for task mutations.
- Update `docs/users-guide.md` with:
  - how to run the frontend preview against the live API in development;
  - required environment variables or proxy configuration;
  - the current behavioural limits of the preview seam.
- Mark roadmap item `4.4.2` done in `docs/roadmap.md` only after tests,
  documentation, and quality gates pass.

Go/no-go: do not close the roadmap item until the documented preview path,
contract tests, and implementation all describe the same behaviour.

## Concrete steps

All commands should run from the repository root. Long-running commands must be
run with `set -o pipefail` and `tee` so failures remain inspectable.

Planned verification commands for the implementation phase:

```bash
set -o pipefail; make frontend-install 2>&1 | tee /tmp/4-4-2-frontend-install.log
set -o pipefail; make frontend-lint 2>&1 | tee /tmp/4-4-2-frontend-lint.log
set -o pipefail; make frontend-typecheck 2>&1 | tee /tmp/4-4-2-frontend-typecheck.log
set -o pipefail; make frontend-test 2>&1 | tee /tmp/4-4-2-frontend-test.log
set -o pipefail; make frontend-test-a11y 2>&1 | tee /tmp/4-4-2-frontend-test-a11y.log
set -o pipefail; make frontend-e2e 2>&1 | tee /tmp/4-4-2-frontend-e2e.log
set -o pipefail; make check-fmt 2>&1 | tee /tmp/4-4-2-check-fmt.log
set -o pipefail; make lint 2>&1 | tee /tmp/4-4-2-lint.log
set -o pipefail; make test TEST_FLAGS='--profile long --all-targets --all-features' 2>&1 | tee /tmp/4-4-2-test.log
set -o pipefail; make fmt 2>&1 | tee /tmp/4-4-2-fmt.log
set -o pipefail; PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/4-4-2-markdownlint.log
set -o pipefail; make nixie 2>&1 | tee /tmp/4-4-2-nixie.log
```

Expected implementation order:

1. Lock the contract and dependency strategy in Stage A.
2. Harden the backend HTTP contract in Stage B.
3. Add the preview auth seam and frontend HTTP seam in Stage C.
4. Prove the contract and seam with tests in Stage D.
5. Update docs and close the roadmap item in Stage E.

## Idempotence and recovery

- The planned verification commands above are safe to rerun.
- Golden fixtures must normalize dynamic values such as timestamps and request
  identifiers before comparison so retries do not create false failures.
- Browser-preview configuration should fail closed with clear errors when the
  development auth seam is misconfigured.
- If the shared `actix-v2a` dependency changes while this milestone is in
  flight, update the Decision Log and re-run the contract inventory before
  proceeding.

## Interfaces and dependencies

Planned interfaces and ownership boundaries:

- Backend:
  - `src/http_api/routes/tasks.rs` remains the driving adapter for task routes;
  - `src/http_api/auth.rs` remains the auth and request-context seam unless a
    narrow preview-only helper is added;
  - `src/http_api/response.rs` and `src/http_api/error/` own the transport
    envelope and error-shape integration.
- Frontend:
  - `frontend-pwa/src/task_slice/ports/task-slice-gateway.ts` owns the
    slice-facing contract;
  - `frontend-pwa/src/task_slice/adapters/http/http-task-gateway.ts` owns fetch
    calls, envelope parsing, and auth-seam consumption;
  - route modules and UI components continue consuming hooks and ports only.

Expected dependency posture:

- Required:
  - roadmap `4.2.1`;
  - the phase 4 `actix-v2a` core HTTP contract dependency, directly or through
    a compatibility seam;
  - existing `pg-embedded-setup-unpriv` test infrastructure.
- Explicitly not required:
  - the phase 4 `actix-v2a` SSE dependency;
  - Podbot hosted-session work;
  - Frankie review-adapter work.

## Artifacts and notes

Implementation should capture the following evidence in this document as work
proceeds:

- the final task contract fixture set and where it lives;
- which shared transport primitives were adopted directly versus wrapped;
- the exact development auth seam chosen;
- the final idempotency decision for task create and transition;
- pass or fail summaries for the planned quality gates.

## Revision note

- 2026-04-10: Initial draft created from roadmap item `4.4.2`, RFC 0002,
  `docs/corbusier-api-design.md`, the completed `4.4.1` and `4.2.1` execplans,
  and inspection of the current frontend and HTTP task adapter code.
