# Create the `frontend-pwa/` workspace and task route shell (roadmap 4.4.1)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETED

Current roadmap numbering places this work at `4.4.1` in
[docs/roadmap.md](../roadmap.md).

Execution began after user approval on 2026-04-08.

## Purpose / big picture

Implement roadmap item `4.4.1`, so Corbusier gains a repository-owned
`frontend-pwa/` workspace that boots locally, renders the narrow task create
and task detail routes, and proves the first browser-facing vertical-slice
shell without requiring backend changes.

After this change, a contributor can:

1. Install and run a repository-owned frontend workspace from
   `frontend-pwa/`.
2. Open a task-create route, submit issue metadata through a fixture-backed
   adapter, and land on a task-detail route.
3. View the task shell with origin, state, timestamps, and branch or
   pull-request placeholders shaped to match the current HTTP task contract.

Observable success means:

1. `frontend-pwa/` boots locally through repository-owned scripts and Makefile
   targets.
2. The task create and task detail routes render through the production PWA
   shell using fixture adapters only.
3. Unit/component and browser-path tests prove happy and unhappy paths without
   needing live API, Podbot, or Frankie services.

## Constraints

- Preserve hexagonal boundaries inside the frontend slice:
  - Pure task-slice domain and view-model code must stay framework-agnostic.
  - Ports must define the slice-owned contracts for create and detail data.
  - React routes, TanStack Query wiring, and fixture or HTTP clients must stay
    in adapters.
  - Components must not own raw transport mapping or fetch logic.
- Keep scope strictly to roadmap item `4.4.1`:
  - create the repository-owned `frontend-pwa/` workspace;
  - establish the app shell, providers, design tokens, and localization runtime
    needed by the narrow task routes;
  - render fixture-backed task create and task detail routes;
  - do not implement live HTTP mutations, the development auth seam,
    Server-Sent Events (SSE), branch or pull-request mutation actions,
    dashboard views, conversation surfaces, Podbot actions, or Frankie
    integrations.
- Align the workspace with RFC 0001 and RFC 0002:
  - use Bun, Vite, React 19, TypeScript, TanStack Router, Tailwind CSS v4, and
    DaisyUI v5 as the baseline stack unless the implementation discovers a
    concrete blocker;
  - prefer adapting Wildside's `packages/tokens` build and runtime approach
    over inventing a Corbusier-specific token pipeline from scratch;
  - keep state management to route-local state plus TanStack Query;
  - do not add Dexie, XState, service workers, offline persistence, or
    Zustand-backed cross-cutting state unless a documented blocker appears.
- Shape fixture contracts from the existing `4.2.1` task HTTP surface rather
  than inventing a parallel task schema. The current task routes expose:
  - `POST /api/v1/tasks`
  - `GET /api/v1/tasks/{task_id}`
  - task fields `id`, `origin`, `branch_ref`, `pull_request_ref`, `state`,
    `created_at`, and `updated_at`
- Respect the repository quality and documentation rules:
  - add or update Makefile targets for frontend lint, type-check, test, and
    browser verification so contributors can stay on the repository-wide `make`
    workflow;
  - keep files below the repository's 400-line rule by splitting app shell,
    slice domain, adapters, and tests into focused modules;
  - update `docs/corbusier-design.md` with the adopted frontend slice boundary
    and fixture/live adapter split;
  - update `docs/users-guide.md` with the local run path and the current
    fixture-backed browser behaviour;
  - mark roadmap item `4.4.1` done in `docs/roadmap.md` only after all feature
    and documentation gates pass.
- Testing expectations:
  - validate frontend logic with focused unit and component tests;
  - validate the create-to-detail browser path with behavioural browser tests;
  - if any Rust-side preview or fixture-serving support code is introduced,
    cover it with `rstest` unit tests and `rstest-bdd` behavioural tests;
  - keep the local full-stack path compatible with
    `pg-embedded-setup-unpriv`, but do not make live Postgres-backed browser
    execution a done criterion for `4.4.1`; that belongs to later live-contract
    work.

## Tolerances (exception triggers)

- Scope: stop and escalate if the slice requires more than one additional route
  family beyond task create and task detail, or if the implementation exceeds
  roughly 35 changed files or 2,500 net lines.
- Contract: stop and escalate if `4.4.1` cannot remain fixture-backed and would
  require backend API changes, auth-shape changes, or shared `actix-v2a`
  transport work from `4.4.2`.
- Tooling: stop and escalate if Bun or Vite integration forces a repository
  restructuring broader than adding `frontend-pwa/`, root ignore rules, and
  focused Makefile targets.
- State: stop and escalate if route-local state plus TanStack Query proves
  insufficient for the narrow slice and an extra state-management library seems
  necessary.
- Testing: stop and escalate if useful browser-path coverage cannot be achieved
  without settling the long-term auth model or without live backend coupling.
- Dependency: stop and escalate if implementation needs libraries outside the
  RFC-approved frontend stack, except for focused test helpers or thin routing
  support.

## Risks

- Risk: the repository currently has no JavaScript or TypeScript workspace, so
  adding one may leak ad hoc tooling into the root instead of staying contained
  in `frontend-pwa/`. Severity: medium. Likelihood: high. Mitigation: keep the
  frontend self-contained and expose it through explicit Makefile targets.
- Risk: route-shell fixtures can drift away from the real task data transfer
  object (DTO) already exposed by `4.2.1`. Severity: medium. Likelihood:
  medium. Mitigation: define shared frontend task types from the current
  `src/http_api/routes/tasks.rs` request and response shapes, and document any
  deliberate simplifications.
- Risk: React route modules may absorb domain mapping, validation, and fixture
  logic, eroding the requested hexagonal separation. Severity: high.
  Likelihood: medium. Mitigation: create explicit slice ports and adapters from
  the start, even for fixture-backed mode.
- Risk: the first slice can bloat into `4.4.2` or `4.4.3` by attempting live
  HTTP, auth, or state-transition actions too early. Severity: high.
  Likelihood: medium. Mitigation: treat fixture-backed create/detail-only flow
  as the hard scope boundary for done.
- Risk: browser tests may become brittle if they assert implementation details
  instead of route-level behaviour. Severity: medium. Likelihood: medium.
  Mitigation: keep end-to-end tests focused on navigation, visible task fields,
  validation errors, and not-found states.

## Progress

- [x] (2026-04-08 11:50Z) Reviewed `docs/roadmap.md` item `4.4.1`, RFC 0001,
  RFC 0002, existing ExecPlan conventions, and the repository planning rules.
- [x] (2026-04-08 11:55Z) Verified the current repository state: no existing
  `frontend-pwa/` workspace, no package-manager metadata, and no root frontend
  Makefile targets yet exist.
- [x] (2026-04-08 12:00Z) Inspected the completed `4.2.1` HTTP task routes to
  ground fixture contracts in the current `POST /api/v1/tasks` and
  `GET /api/v1/tasks/{task_id}` shapes.
- [x] (2026-04-08 12:05Z) Authored the initial ExecPlan draft in this file.
- [x] (2026-04-08 20:30Z) User approved execution of the plan.
- [x] (2026-04-08 20:40Z) Bootstrapped `frontend-pwa/` with Bun, Vite, React
  19, TypeScript, Tailwind CSS v4, DaisyUI v5, Playwright, Vitest, and
  repository-owned `make` targets.
- [x] (2026-04-08 20:48Z) Implemented the fixture-backed task-slice domain,
  ports, adapters, app providers, and task create/detail routes.
- [x] (2026-04-08 20:58Z) Added unit/component coverage and browser-path tests
  for create success, invalid create input, and task-detail not-found states.
- [x] (2026-04-08 21:00Z) Updated design and user docs, marked roadmap item
  `4.4.1` complete, and reran the planned frontend/documentation gates.

## Surprises & Discoveries

- The repository is currently Rust-only at the workspace root: there is no
  `package.json`, `bun.lockb`, `tsconfig.json`, `vite.config.*`,
  `playwright.config.*`, or `frontend-pwa/` directory yet.
- The roadmap dependency `4.2.1` is already complete, and the task HTTP
  surface is narrower than the full vertical slice: it already supports create,
  detail, transition, branch association, and pull-request association, but
  `4.4.1` should only mirror create/detail shape and defer live mutations.
- The current task data transfer object (DTO) in
  `src/http_api/routes/tasks.rs` already provides the exact detail-shell fields
  needed for the first route shell: `id`, `origin`, `branch_ref`,
  `pull_request_ref`, `state`, `created_at`, and `updated_at`.
- The root `Makefile` currently only covers Rust and documentation gates, so
  `4.4.1` must introduce a repository-native way to run frontend quality checks
  without bypassing `make`.
- Comparative review outcome:
  - Wildside's token package already matches the planned frontend stack: it
    builds CSS variables through Style Dictionary, derives Tailwind and DaisyUI
    presets, ships token-resolution helpers for Node and browser contexts, and
    enforces theme contrast in-package.
  - `figtok` is better understood as a Figma Tokens Studio ingestion CLI and
    token parser. It can emit CSS variables and JSON, but it is not already
    aligned with Bun, Vite, Tailwind, DaisyUI, or the repository's existing
    TypeScript build tooling.

## Decision Log

- Decision: create `frontend-pwa/` as a repository-owned workspace with its own
  toolchain metadata, rather than treating the external mockup as a runtime
  dependency. Rationale: aligns with RFC 0001 and keeps the production path
  reviewable in-repo. Date/Author: 2026-04-08 / plan author.
- Decision: model the task slice as a small frontend hexagon with pure task
  types and validation helpers, slice-owned ports, and fixture or HTTP
  adapters. Rationale: satisfies the repository's architectural boundary rules
  and keeps later `4.4.2` and `4.4.3` work additive instead of invasive.
  Date/Author: 2026-04-08 / plan author.
- Decision: treat fixture-backed create/detail flow as the only required
  behaviour for `4.4.1`; live auth, idempotency, error-envelope contract
  hardening, and task-state mutation stay in later roadmap steps. Rationale:
  this matches RFC 0002's recommended order and keeps the first browser slice
  narrow. Date/Author: 2026-04-08 / plan author.
- Decision: mirror the existing Rust task request and response data transfer
  objects (DTOs) in the frontend slice rather than inventing mockup-only
  transport shapes. Rationale: reduces drift between fixture mode and the
  already-landed HTTP contract. Date/Author: 2026-04-08 / plan author.
- Decision: add frontend quality gates through Makefile targets instead of
  relying on undocumented direct Bun commands. Rationale: follows repository
  command policy and keeps CI or local workflows reviewable. Date/Author:
  2026-04-08 / plan author.
- Decision: prefer Wildside's token-management approach as the default reuse
  target for `frontend-pwa/`, and do not introduce a new Corbusier-local token
  compiler unless adaptation proves infeasible. Rationale: Wildside already
  produces the concrete outputs this slice needs (CSS variables, Tailwind
  preset, DaisyUI theme, runtime token helpers, and contrast validation),
  whereas `figtok` is a generic Figma Tokens Studio serializer better suited to
  a future design-ingestion workflow than to the immediate `4.4.1` workspace
  shell. Date/Author: 2026-04-08 / plan author.
- Decision: treat `figtok` as a conditional fallback only if Corbusier later
  chooses Figma Tokens Studio export files as the canonical token source.
  Rationale: it already handles standard, composition, and shadow tokens plus
  CSS-variable output, but it would still need extra adaptation for the
  Tailwind, DaisyUI, Bun, and browser-runtime needs already solved in
  Wildside's package. Date/Author: 2026-04-08 / plan author.

## Outcomes & Retrospective

Planning outcome: the `4.4.1` implementation sequence, boundaries, and quality
gates are now captured in one execution document. Delivery outcomes,
deviations, and lessons learned will be added after execution.

Delivery outcome: Corbusier now has a repository-owned `frontend-pwa/`
workspace with a fixture-backed task create route, task detail route, route
shell providers, design tokens, localization runtime, and explicit `make`
targets for install, lint, type-check, unit tests, and browser-path tests.

Retrospective:

- Keeping the task gateway behind a slice-owned port made the fixture adapter
  and stub HTTP adapter additive instead of forcing route rewrites.
- Vitest configuration needed an explicit include list so Playwright specs did
  not get treated as unit tests.
- TanStack Router tests were stable once the memory-history router was loaded
  inside the test harness before assertions ran.

## Context and orientation

Current repository state relevant to `4.4.1`:

- Root workspace is a Rust project with docs and tests, but no frontend
  workspace yet.
- Completed roadmap item `4.2.1` already exposes authenticated task routes at:
  - `POST /api/v1/tasks`
  - `GET /api/v1/tasks/{task_id}`
  - `PUT /api/v1/tasks/{task_id}/state`
  - `PUT /api/v1/tasks/{task_id}/branch`
  - `PUT /api/v1/tasks/{task_id}/pull-request`
- RFC 0002 narrows `4.4.1` to shell creation and fixture-backed task routes,
  with live contract hardening explicitly deferred to `4.4.2`.
- `docs/users-guide.md` already documents the HTTP API surface, so the frontend
  plan can reuse that terminology instead of inventing new names.

Target workspace shape for this milestone:

```plaintext
frontend-pwa/
├── package.json
├── bun.lock
├── tsconfig.json
├── vite.config.ts
├── playwright.config.ts
├── biome.json
├── src/
│   ├── app/
│   │   ├── providers/
│   │   ├── router/
│   │   └── shell/
│   ├── design/
│   │   ├── tokens/
│   │   └── theme/
│   ├── i18n/
│   │   ├── locales/
│   │   └── runtime/
│   ├── task_slice/
│   │   ├── domain/
│   │   ├── ports/
│   │   ├── adapters/
│   │   │   ├── fixture/
│   │   │   └── http/
│   │   ├── application/
│   │   └── ui/
│   └── routes/
│       ├── __root.tsx
│       ├── tasks.new.tsx
│       └── tasks.$taskId.tsx
└── tests/
    ├── component/
    └── e2e/
```

The `http/` adapter may exist as a stub or interface-only module in this
milestone so that `4.4.2` and `4.4.3` can attach live transport logic without
rewriting the route shell.

## Plan of work

### Stage A: bootstrap the workspace and repository hooks

Create the repository-owned PWA workspace and baseline tooling:

- Add `frontend-pwa/` with Bun, Vite, React 19, TypeScript, and TanStack Router
  scaffolding.
- Add Tailwind CSS v4, DaisyUI v5, and the minimum design-token scaffolding
  needed to render the task routes.
- Start from Wildside's token package conventions where feasible:
  - CSS variables generated from source tokens;
  - derived Tailwind preset and DaisyUI theme outputs;
  - token-resolution helpers for runtime use;
  - built-in contrast validation for theme safety.
- Only evaluate `figtok` in this stage if the design source of truth is pushed
  towards Figma Tokens Studio exports and Wildside's package cannot consume the
  same source material with modest adaptation.
- Add baseline i18n runtime and an English locale bundle for UI chrome.
- Add root-level integration points:
  - `.gitignore` entries for frontend artefacts;
  - Makefile targets for frontend install, lint, type-check, unit tests, and
    browser tests;
  - documentation for the local run path.

Go/no-go: do not proceed until the workspace boots locally and repository-owned
commands exist to run its quality stack.

### Stage B: define the task-slice domain, ports, and fixture adapters

Define the narrow slice contract before building screens:

- Add pure frontend task types mirroring the existing create/detail HTTP
  request and response shapes.
- Add view-model helpers for:
  - issue-origin field formatting,
  - state label or badge mapping,
  - timestamp formatting boundaries, and
  - placeholder branch or pull-request display.
- Define slice ports, for example:
  - `TaskCreatePort`
  - `TaskDetailPort`
  - a combined `TaskSliceGateway`
- Implement fixture adapters that cover:
  - successful task creation and navigation payloads;
  - validation failures for incomplete or malformed input;
  - task-not-found detail loading;
  - representative edge cases such as missing optional description, labels, or
    references.

Go/no-go: do not proceed until route modules can depend on ports only, with no
raw fixture JSON or transport mapping inside React components.

### Stage C: implement the narrow route shell

Build the visible vertical-slice shell without live transport coupling:

- Add the root app shell, provider composition, and router wiring.
- Implement the task-create route with a form matching the current create-task
  request fields needed by the slice.
- Submit task-create requests through the fixture gateway and navigate to the
  task-detail route on success.
- Implement the task-detail route shell to render:
  - task identity;
  - issue origin;
  - task state;
  - creation and update timestamps;
  - branch and pull-request sections in read-only or placeholder form suitable
    for later live association work.
- Add unhappy-path views:
  - client-side validation feedback;
  - fixture-simulated submission failure;
  - task-detail not-found state.

Go/no-go: do not proceed until a contributor can perform the full
fixture-backed create-to-detail loop in the browser.

### Stage D: prove the slice with tests

Add focused coverage for both logic and behaviour:

- Frontend unit/component coverage:
  - task-field validation and mapping helpers;
  - fixture-adapter behaviours;
  - route rendering for happy path, validation errors, and not-found detail.
- Browser behavioural coverage:
  - create a task from issue metadata and land on detail;
  - reject invalid create input;
  - show a not-found detail state for an unknown task id.
- Rust-side test coverage only if implementation introduces Rust support code:
  - use `rstest` for unit or integration helpers;
  - use `rstest-bdd` for observable preview-path behaviour.
- Confirm the local developer path remains compatible with later Postgres-backed
  API testing through `pg-embedded-setup-unpriv`, while keeping live backend
  execution outside the `4.4.1` done criteria.

Go/no-go: proceed to documentation only when unit/component and browser-path
tests pass from repository-owned commands.

### Stage E: document the slice boundary and close the roadmap item

- Update `docs/corbusier-design.md` with:
  - the repository-owned frontend workspace boundary;
  - the frontend task-slice hexagonal split;
  - the explicit fixture-first, live-transport-later adoption path.
- Update `docs/users-guide.md` with:
  - how to run the new PWA locally;
  - what the current task create/detail browser slice does;
  - what remains intentionally deferred to later steps.
- Mark roadmap item `4.4.1` done in `docs/roadmap.md` only after all quality
  gates pass.

Go/no-go: do not close the roadmap item until docs, tests, and repository
commands all reflect the shipped behaviour.

## Concrete steps

All commands should run from the repository root.

Planned repository-owned commands and gates for this milestone:

```bash
set -o pipefail; make frontend-install 2>&1 | tee /tmp/4-4-1-frontend-install.log
set -o pipefail; make frontend-audit 2>&1 | tee /tmp/4-4-1-frontend-audit.log
set -o pipefail; make frontend-lint 2>&1 | tee /tmp/4-4-1-frontend-lint.log
set -o pipefail; make frontend-typecheck 2>&1 | tee /tmp/4-4-1-frontend-typecheck.log
set -o pipefail; make frontend-test 2>&1 | tee /tmp/4-4-1-frontend-test.log
set -o pipefail; make frontend-test-a11y 2>&1 | tee /tmp/4-4-1-frontend-test-a11y.log
set -o pipefail; make frontend-localizability 2>&1 | tee /tmp/4-4-1-frontend-localizability.log
set -o pipefail; make frontend-semantic 2>&1 | tee /tmp/4-4-1-frontend-semantic.log
set -o pipefail; make frontend-e2e 2>&1 | tee /tmp/4-4-1-frontend-e2e.log
set -o pipefail; make fmt 2>&1 | tee /tmp/4-4-1-fmt.log
set -o pipefail; PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/4-4-1-markdownlint.log
set -o pipefail; make nixie 2>&1 | tee /tmp/4-4-1-nixie.log
```

Expected implementation order:

1. Land Stage A and confirm the workspace boots.
2. Land Stage B so route modules depend on slice ports rather than raw fixture
   data.
3. Land Stage C for the visible create/detail shell.
4. Land Stage D for logic and browser-path coverage.
5. Land Stage E for documentation updates and roadmap closure.
