# Stabilize the slice transport contract and development auth seam (roadmap 4.4.2)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

Current roadmap numbering places this work at `4.4.2` in
[docs/roadmap.md](../roadmap.md).

Execution phase does not begin until this draft is explicitly approved.

## Purpose / big picture

Implement roadmap item `4.4.2`, so the first repository-owned frontend slice
can rely on a stable task transport contract and a repeatable local browser
authentication seam without prematurely freezing the long-term production auth,
pagination, or Server-Sent Events (SSE) model.

After this change, a contributor can:

1. Rely on stable create, detail, and state-transition task endpoint
   contracts, including task detail fields already rendered by the slice.
2. Run the repository-owned `frontend-pwa/` against the current HTTP API
   through a documented development-only auth seam.
3. Reuse golden transport fixtures and contract tests as the base for roadmap
   item `4.4.3`, rather than re-deciding request, response, and error shapes
   during the first live browser-path implementation.

Observable success means:

1. Task create, task detail, and task transition routes expose contract-tested
   success and error responses aligned with the adopted phase 4 `actix-v2a`
   core HTTP primitives where they touch the slice.
2. The browser-facing development auth seam is documented, repeatable, and
   validated without depending on hosted-session, Frankie, pagination, or SSE
   work.
3. The frontend slice has a real HTTP adapter and preview configuration ready
   for roadmap `4.4.3`, while fixture mode remains available and the live route
   switch itself stays scoped to `4.4.3`.

## Constraints

- Preserve hexagonal boundaries in both backend and frontend work:
  - `src/task/` remains the owning bounded context for task lifecycle rules.
  - `src/http_api/` remains a driving adapter that maps request and response
    contracts but does not absorb task orchestration.
  - `frontend-pwa/src/task_slice/domain` stays transport-agnostic.
  - `frontend-pwa/src/task_slice/ports` defines the slice-owned browser
    contract, and `frontend-pwa/src/task_slice/adapters/http` implements the
    live transport mapping.
  - React routes and UI components must not own raw HTTP, auth-header, or
    error-envelope parsing logic.
- Keep scope strictly to roadmap item `4.4.2`:
  - stabilize the slice-relevant HTTP contract for task create, detail, and
    transition flows;
  - preserve branch and pull-request references on task detail payloads so
    later steps can render them without reshaping the detail contract;
  - adopt shared `actix-v2a` `error`, `idempotency`, and `openapi` primitives
    where they materially affect the slice;
  - add a development-only browser auth seam for local preview;
  - do not implement list endpoints, pagination, SSE, hosted-session flows,
    Frankie review actions, or the full live route wiring promised by `4.4.3`.
- Treat the development auth seam as temporary and local-only:
  - acceptable forms are a same-origin development proxy or development-only
    bearer-token injection as described in RFC 0002 §5.4;
  - do not freeze the long-term production login, session cookie, API-key, or
    browser identity model in this roadmap item.
- Reconcile the implementation with the current repository state:
  - `frontend-pwa/src/main.tsx` is still fixture-first;
  - `frontend-pwa/src/task_slice/adapters/http/http-task-gateway.ts` is a stub;
  - `src/http_api/auth.rs` currently accepts bearer JWTs only;
  - `src/http_api/response.rs` and `src/http_api/error/` do not yet reflect the
    error payload shape described in `docs/corbusier-api-design.md`.
- Use repository-native quality gates and test patterns:
  - Rust unit and integration coverage uses `rstest`;
  - behavioural HTTP coverage uses `rstest-bdd` where observable API behaviour
    is under test;
  - local PostgreSQL-backed verification uses `pg-embedded-setup-unpriv`;
  - frontend adapter and preview coverage use the existing Vitest and
    Playwright harnesses where applicable;
  - all validation commands must run through `make` targets, with output piped
    through `tee` during implementation so truncation does not hide failures.
- Documentation updates are part of done:
  - record transport and auth-seam decisions in
    `docs/corbusier-design.md` and `docs/corbusier-api-design.md`;
  - update `docs/users-guide.md` with the local preview/auth workflow and any
    user-visible API-contract changes;
  - mark roadmap item `4.4.2` done in `docs/roadmap.md` only after all code,
    test, and documentation gates pass.

## Tolerances (exception triggers)

- Scope: stop and escalate if the work expands beyond roughly 35 changed files
  or 2,500 net lines, or if it starts pulling task-list or project-list
  projections into the slice.
- Dependency: stop and escalate if the phase 4 `actix-v2a` core HTTP contract
  primitives are unavailable, materially incompatible with the current API
  surface, or would require a broader cross-repository migration than this step
  can absorb.
- Auth: stop and escalate if the development auth seam cannot be made
  repeatable without simultaneously implementing the long-term production login
  or cookie/session subsystem.
- Contract: stop and escalate if stabilizing the slice contract requires
  reshaping unrelated conversation or tool routes, or if the task contract
  cannot be reconciled between `docs/corbusier-api-design.md`, RFC 0002, and
  the shipped `4.2.1` routes.
- Idempotency: stop and escalate if the adopted idempotency primitive requires a
  durable replay or deduplication store broader than the task slice and current
  HTTP API can reasonably support in this step.
- Testing: stop and escalate if meaningful backend contract coverage cannot be
  achieved with `rstest`, `rstest-bdd`, and `pg-embedded-setup-unpriv`, or if
  preview-path verification requires external hosted services.

## Risks

- Risk: the current HTTP response envelope
  (`success`/`data`/`error`/`metadata`) diverges from the API design's desired
  error schema (`code`, `message`, optional `traceId`, optional `details`).
  Severity: high. Likelihood: high. Mitigation: treat response and error-shape
  hardening as the first backend stage, and lock it behind golden fixtures
  before touching the frontend adapter.
- Risk: `src/http_api/auth.rs` currently supports only bearer JWT extraction,
  while `docs/corbusier-api-design.md` recommends a browser-friendly cookie
  default for the long term. Severity: high. Likelihood: medium. Mitigation:
  document the preview auth seam as explicitly temporary, and isolate the
  development-only mechanism from the production auth decision.
- Risk: the frontend can drift from the stabilized contract because the current
  task slice is fixture-backed and the HTTP adapter still throws `unavailable`.
  Severity: medium. Likelihood: high. Mitigation: create shared golden fixtures
  and adapter contract tests so the frontend maps the same payloads and error
  cases as the Rust HTTP tests.
- Risk: idempotency work can sprawl into roadmap item `8.2.1`, which explicitly
  adds task-creation mutation headers and conflict handling for broader live
  projections. Severity: medium. Likelihood: medium. Mitigation: keep `4.4.2`
  to contract stabilization and backend capability, leaving broader live-query
  adoption to later roadmap items.
- Risk: turning on the live HTTP adapter too early would collapse `4.4.2` into
  `4.4.3`. Severity: high. Likelihood: medium. Mitigation: keep fixture mode as
  the default route shell during this step, and validate the browser auth seam
  through explicit preview configuration or targeted smoke coverage.
- Risk: OpenAPI adoption may require new crates or validation tooling not yet
  present in the workspace. Severity: medium. Likelihood: medium. Mitigation:
  keep the OpenAPI work slice-specific and stop if it becomes a repository-wide
  schema programme rather than a transport-contract stabilization task.

## Progress

- [x] (2026-04-10 22:32Z) Reviewed roadmap item `4.4.2`, RFC 0002 §5.3-§5.4,
  `docs/corbusier-api-design.md` §HTTP API surface, pagination, SSE, and error
  contracts, and existing ExecPlan conventions.
- [x] (2026-04-10 22:32Z) Inspected the current repository state across
  `frontend-pwa/`, `src/http_api/`, `src/task/`, and the existing HTTP API and
  frontend test harnesses.
- [x] (2026-04-10 22:32Z) Authored the initial ExecPlan draft in this file.
- [ ] Await user approval of this ExecPlan before implementation.
- [ ] Execute stages A-E, updating this section, the decision log, and the
  retrospective as the work proceeds.

## Surprises & Discoveries

- Roadmap item `4.4.1` intentionally left the live adapter seam in place:
  `frontend-pwa/src/task_slice/adapters/http/http-task-gateway.ts` already
  exists, but it is a stub that throws `TaskGatewayError('unavailable', ...)`.
- The current frontend entrypoint (`frontend-pwa/src/main.tsx`) always mounts
  the fixture gateway, so `4.4.2` needs an explicit preview-mode switch rather
  than an incidental code-path change.
- The current backend task detail payload already includes `branch_ref` and
  `pull_request_ref`, which means `4.4.2` can preserve the detail-screen
  contract needed by `4.4.4` without adding those mutations yet.
- The shipped HTTP error shape is still Corbusier-local: task routes currently
  return a generic envelope plus `ErrorPayload { code, message }`, with the
  request identifier only inside response metadata. That differs materially
  from the error-contract guidance in `docs/corbusier-api-design.md`.
- No current repository code or dependency declarations reference `actix-v2a`,
  reusable OpenAPI fragments, or shared idempotency helpers. That dependency is
  therefore a real integration checkpoint, not merely a documentation note.

## Decision Log

- Decision: treat `4.4.2` as contract hardening plus development-preview
  plumbing, not as the roadmap item that switches the main task routes to live
  HTTP by default. Rationale: RFC 0002 assigns the first live browser path to
  `4.4.3`, so this step should leave that activation as a small, additive
  follow-on. Date/Author: 2026-04-10 / plan author.
- Decision: the slice-relevant contract for this step covers task create, task
  detail, and task transition endpoints, while task detail continues to carry
  branch and pull-request references already present in the DTO. Rationale:
  this matches RFC 0002 §5.3 and preserves the detail contract needed by later
  slice steps without broadening into `4.4.4`. Date/Author: 2026-04-10 / plan
  author.
- Decision: keep the browser auth seam explicitly development-only and
  same-origin. Rationale: RFC 0002 requires a repeatable local run path but
  deliberately does not settle the production login or browser identity model.
  Date/Author: 2026-04-10 / plan author.
- Decision: use golden transport fixtures as the shared source of truth between
  Rust HTTP tests and frontend adapter tests. Rationale: this is the lowest
  risk way to keep the fixture-first slice aligned with the live contract
  before `4.4.3` enables the live browser path. Date/Author: 2026-04-10 / plan
  author.

## Outcomes & Retrospective

Planning outcome: the repository now has a draft implementation sequence for
roadmap item `4.4.2` that keeps scope narrow, makes the dependency and risk
checkpoints explicit, and preserves the approval gate before any implementation
work begins.

Delivery outcomes, deviations, validation evidence, and follow-up lessons will
be recorded here after execution.

## Context and orientation

Current repository state relevant to `4.4.2`:

- Completed roadmap item `4.2.1` already exposes:
  - `POST /api/v1/tasks`
  - `GET /api/v1/tasks/{task_id}`
  - `PUT /api/v1/tasks/{task_id}/state`
  - `PUT /api/v1/tasks/{task_id}/branch`
  - `PUT /api/v1/tasks/{task_id}/pull-request`
- The frontend slice delivered by `4.4.1` is still fixture-first:
  - route shell and UI live under `frontend-pwa/src/routes/` and
    `frontend-pwa/src/task_slice/ui/`;
  - slice-owned ports live under `frontend-pwa/src/task_slice/ports/`;
  - the HTTP adapter exists but is not implemented;
  - the preview entrypoint still mounts the fixture gateway directly.
- The backend auth layer lives in `src/http_api/auth.rs` and currently enforces
  `Authorization: Bearer <jwt>` using HS256 JWT claims. No browser-oriented
  development seam is documented yet.
- The backend response layer lives in `src/http_api/response.rs` and
  `src/http_api/error/`. It already provides versioned envelopes and request
  identifiers, but not the full error shape recommended by
  `docs/corbusier-api-design.md`.
- Existing test harnesses already provide the right execution homes for this
  work:
  - Rust adapter and service tests under `src/` and `tests/`;
  - BDD HTTP API scenarios under `tests/features/http_api_surface.feature` and
    `tests/http_api_surface_*`;
  - Postgres-backed HTTP API tests under `tests/postgres/http_api_surface_*`;
  - frontend unit/component tests under `frontend-pwa/src/routes/` and
    `frontend-pwa/src/task_slice/adapters/`;
  - Playwright coverage under `frontend-pwa/tests/e2e/`.

## Plan of work

### Stage A: lock the slice contract and dependency intake

Establish the exact transport surface that `4.4.2` will stabilize before any
implementation changes:

- Reconcile the task slice's current needs across:
  - `docs/rfcs/0002-deliver-the-first-front-end-vertical-slice.md` §5.3-§5.4;
  - `docs/corbusier-api-design.md` §HTTP API surface, pagination, SSE, and
    error contracts;
  - the shipped `src/http_api/routes/tasks.rs` handlers and DTOs;
  - the current frontend task domain types in
    `frontend-pwa/src/task_slice/domain/task.ts`.
- Determine exactly which phase 4 `actix-v2a` primitives will be imported or
  adapted for:
  - error envelope and error-code compatibility;
  - idempotency handling on slice-relevant mutation endpoints;
  - reusable OpenAPI fragments for the task routes touched by the slice.
- Decide whether the slice-owned frontend port must expand now to cover task
  transition semantics, while still keeping the live UI activation deferred to
  `4.4.3`.
- Capture the approved contract deltas in this ExecPlan and in the relevant
  design docs before broader code changes begin.

Go/no-go checkpoint: do not proceed until the adopted `actix-v2a` dependency
surface and the exact slice-scoped contract delta are both explicit.

### Stage B: harden the backend task transport contract

Refactor the HTTP adapter so the task slice can depend on stable task payload,
error, and idempotency semantics:

- Update `src/http_api/response.rs`, `src/http_api/error/`, and
  `src/http_api/routes/tasks.rs` to use the adopted shared error primitives,
  including structured request-trace correlation and any required `details`
  payloads.
- Introduce the slice-relevant idempotency boundary for task mutations using
  the adopted shared primitive, but keep the implementation scoped to the task
  routes needed by the slice.
- Preserve the current detail fields (`id`, `origin`, `branch_ref`,
  `pull_request_ref`, `state`, `created_at`, `updated_at`) so the frontend
  detail card contract stays additive rather than disruptive.
- Add or expose reusable OpenAPI fragments only for the stabilized task routes
  touched by the slice.
- Keep pagination and SSE explicitly out of scope for done, even if the
  supporting docs discuss them.

Go/no-go checkpoint: do not proceed until task create, detail, and transition
routes have stable JSON fixtures for both success and unhappy-path responses.

### Stage C: implement the frontend HTTP adapter and development auth seam

Make the browser slice transport-ready without collapsing this step into the
full live workflow of `4.4.3`:

- Implement `frontend-pwa/src/task_slice/adapters/http/http-task-gateway.ts`
  against the stabilized task contract and map transport failures into
  slice-owned `TaskGatewayError` variants.
- Add any required transport types, query wiring, or configuration helpers so
  the task slice can switch between fixture and HTTP adapters without changing
  route or component structure.
- Introduce a documented development-only auth seam, for example:
  - a Vite same-origin proxy to the local Corbusier API, or
  - development-only bearer-token injection for preview mode.
- Keep the auth seam outside the task domain and UI components. The browser
  seam belongs in preview/bootstrap or adapter configuration, not in the route
  modules.
- Ensure fixture mode remains available and predictable, because `4.4.3` still
  owns the first end-to-end live create-detail-transition route.

Go/no-go checkpoint: do not proceed until the frontend can run in a repeatable
preview mode against the authenticated local API without changing the task
slice's domain or UI structure.

### Stage D: validate contract, auth seam, and unhappy paths

Prove the stabilized contract through both backend and frontend tests:

- Rust unit and adapter tests with `rstest` for:
  - task DTO serialization and error mapping;
  - request-context and auth-seam helpers;
  - idempotency header validation and replay or conflict behaviour where
    adopted;
  - OpenAPI fragment or schema wiring where present.
- Behavioural HTTP tests with `rstest-bdd` covering:
  - authenticated task creation through HTTP;
  - authenticated task detail retrieval;
  - task state transition through HTTP;
  - missing or invalid auth;
  - invalid task identifiers or validation failures;
  - idempotency replay or conflict behaviour if the stabilized contract exposes
    it in this step.
- PostgreSQL-backed verification using `pg-embedded-setup-unpriv` for the live
  task HTTP surface, especially when idempotency or durable request replay
  touches persistence concerns.
- Frontend tests for:
  - HTTP adapter success and failure mapping;
  - preview-mode configuration and auth seam behaviour;
  - any targeted browser smoke path needed to prove the documented preview run
    path remains repeatable.

Validation gates to run during implementation:

- `make check-fmt`
- `make lint`
- `make test TEST_FLAGS='--profile long --all-targets --all-features'`
- `make frontend-lint`
- `make frontend-typecheck`
- `make frontend-test`
- `make frontend-e2e`
- `make fmt`
- `PATH=/root/.bun/bin:$PATH make markdownlint`
- `make nixie`

During implementation, run each gate with `set -o pipefail` and `tee` to a
temporary log file so truncated terminal output does not hide failures.

Go/no-go checkpoint: do not proceed to roadmap closure until all applicable
gates pass against the final contract and documentation.

### Stage E: update design and user documentation, then close the roadmap item

Record the finalized contract and preview workflow where future contributors
will look first:

- Update `docs/corbusier-design.md` with the stabilized frontend/backend seam,
  including the boundary between fixture mode, HTTP adapter mode, and the
  development-only auth path.
- Update `docs/corbusier-api-design.md` so the documented task contract matches
  the adopted error, idempotency, and OpenAPI approach actually implemented for
  the slice.
- Update `docs/users-guide.md` with:
  - the local preview workflow;
  - required development auth inputs or defaults;
  - any user-visible error-contract or request expectations relevant to task
    routes.
- Mark roadmap item `4.4.2` done in `docs/roadmap.md` only after the contract,
  tests, preview path, and documentation updates are all complete.

Completion note: after `4.4.2` is complete, roadmap item `4.4.3` should be able
to focus on enabling the live task create-detail-transition UI path rather than
reworking transport or auth decisions.
