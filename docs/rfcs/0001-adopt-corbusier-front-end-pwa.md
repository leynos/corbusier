# RFC 0001: Adopt the Corbusier front-end Progressive Web App

## Preamble

- **RFC number:** 0001
- **Status:** Proposed
- **Created:** 2026-04-01

## 1. Summary

Corbusier should adopt a production Progressive Web App (PWA) frontend based on
the proved-out user-interface and interaction model in
[`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup),
implemented as a first-class workspace inside the Corbusier repository and
integrated with the existing backend through stable HTTP and Server-Sent Events
(SSE) contracts.

The proposal does not recommend copying the mockup wholesale into production.
Instead, it recommends adopting the shared v2a frontend stack, carrying over
the mockup's data-model-driven card architecture, and reusing the Nile Valley
and Wildside patterns that already solve the hard cross-cutting problems:
pagination, idempotent mutations, user-versus-tenant identity, session-aware
API access, and event-stream invalidation.

The result is a Corbusier-owned PWA that preserves the mockup's validated user
experience while aligning with Corbusier's backend architecture, its API and
data-model work, and the Nile Valley deployment model already used elsewhere in
df12 Productions systems.

## 2. Problem

Corbusier currently has a backend design, roadmap, and API extension proposal
that collectively describe the data and orchestration model needed by a rich
frontend, but it does not yet have an accepted repository-level plan for
adopting the Corbusier frontend PWA.

That gap creates several risks:

- The mockup can drift away from the backend contracts that Corbusier are
  converging on.
- A later frontend implementation could regress into a second system design
  exercise instead of reusing the validated work already carried out in
  [`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup).
- Shared Nile Valley concerns such as session identity, relative API routing,
  pagination envelopes, and deterministic retry semantics could be solved
  differently in Corbusier and Wildside, increasing maintenance cost.
- The repository currently lacks a reviewable statement of which Wildside and
  v2a patterns are portable, which are Corbusier-specific, and which remain
  deliberately out of scope.

## 3. Current state

Corbusier already has several strong architectural anchors:

- `docs/corbusier-design.md` defines the orchestration-first backend and its
  domain boundaries.
- `docs/corbusier-api-design.md` already derives a backend-facing frontend
  contract from
  [`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup),
  including projects, tasks, conversations, directives, suggestions,
  governance, identity, pagination, and SSE.
- `docs/roadmap.md` already contains backend work items for OpenAPI, reusable
  pagination, and read-model projections aligned with the mockup.

The proved-out frontend sits outside this repository in
[`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup). That
mockup already establishes:

- the shared version 2a (v2a) stack for df12 Productions PWAs,
- the data-model-driven card architecture used by both Wildside and Corbusier,
- the screen inventory for Corbusier's dashboard, tasks, projects,
  conversations, directives, suggestions, and system pages, and
- the testing and accessibility posture expected of the frontend.

Wildside provides the closest production-side precedent for adopting that
stack. Wildside already documents:

- a PWA architecture built on the same v2a frontend model,
- a backend-compatible PWA data model,
- keyset pagination envelopes suitable for TanStack Query infinite lists,
- idempotency semantics and replay behaviour for mutation endpoints, and
- contract tests covering optimistic concurrency, idempotency conflicts, and
  deterministic replay.

The missing piece is the adoption strategy that connects those assets into a
single implementation direction for Corbusier.

### 3.1. Referenced projects and systems

- [`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup):
  fixture-driven proving ground for the Corbusier frontend screens, component
  composition, and v2a stack choices.
- [`leynos/wildside`](https://github.com/leynos/wildside): closest production
  precedent for the shared Progressive Web App (PWA) patterns reused here,
  especially pagination, idempotency, and frontend-backend contract design.
- [`leynos/nile-valley`](https://github.com/leynos/nile-valley): platform and
  deployment model that hosts df12 Productions web workloads and informs the
  same-origin serving assumptions in this RFC.
- **Version 2a (v2a)**: df12 Productions' shared frontend application pattern
  covering the React, routing, styling, localization, testing, and optional
  local-first state stack used across Corbusier and Wildside mockups.

## 4. Goals and non-goals

- Goals:
  - Adopt a Corbusier-owned PWA workspace derived from the proved-out mockup.
  - Standardize on the shared v2a frontend stack already validated in
    [`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup).
  - Reuse Wildside's established patterns for pagination, idempotency,
    identity, and event-stream contracts where those concerns are the same.
  - Preserve Corbusier-specific domain behaviour and information architecture
    rather than forcing a Wildside-shaped application onto Corbusier.
  - Align the frontend adoption path with Nile Valley's deployment and
    same-origin platform model.
  - Make the migration incremental so fixture-backed screens can be replaced by
    live backend data screen by screen.
- Non-goals:
  - Rebuild Corbusier as a copy of Wildside's map-centric or offline-bundle
    product model.
  - Commit to every optional v2a technology from day one if the concrete
    Corbusier interaction does not yet require it.
  - Freeze the exact component library implementation details for all future
    screens.
  - Define every backend endpoint in this RFC; those details remain in
    `docs/corbusier-api-design.md` and subsequent implementation work.
  - Introduce a separate frontend deployment origin distinct from Corbusier's
    main Nile Valley-served application surface.

## 5. Proposed design

### 5.1. Adopt the mockup as the user experience (UX) and contract proving ground

Corbusier should treat
[`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup) as the
proving ground for:

- route and screen structure,
- visual language and component composition,
- card and detail-view data shapes,
- localization responsibilities, and
- accessibility and testing expectations.

The production frontend should not start from a blank directory. It should
start by importing, adapting, and then progressively replacing the mockup's
fixture-driven modules inside a repository-owned `frontend-pwa/` workspace.

### 5.2. Create a repository-owned `frontend-pwa/` workspace

Corbusier should adopt a dedicated frontend workspace in the main repository,
using the same broad toolchain already validated in
[`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup):

- Bun for package management and scripts,
- Vite for bundling and development,
- React 19 for the single-page runtime,
- TanStack Router for route structure,
- Tailwind CSS v4 and DaisyUI v5 for styling,
- Radix UI primitives for accessible interactions,
- i18next with Fluent bundles for localization,
- Biome, TypeScript, Stylelint, and semantic checks for quality, and
- Playwright plus axe-core for end-to-end and accessibility verification.

This workspace should be the production target. The mockup remains the faster
experimentation environment until each feature is promoted.

### 5.3. Preserve the v2a data-model-driven card architecture

Corbusier should adopt the v2a rule that entities own their localizable names,
descriptions, badges, and semantic descriptors, while translation bundles keep
only UI chrome and formatting strings.

That means:

- task, project, conversation, directive, suggestion, personnel, governance,
  and registry entities arrive with localization-aware fields,
- descriptor registries resolve stable semantic identifiers such as task state,
  priority, and health status into display metadata, and
- frontend components remain presentational rather than reconstructing domain
  meaning from ad hoc strings.

This is already consistent with `docs/corbusier-api-design.md`, which proposes
projection Data Transfer Objects (DTOs) designed to match the mockup cards.

### 5.4. Reuse Wildside patterns where the problem is the same

Corbusier should reuse Wildside's approach for cross-cutting application
concerns that are not domain-specific:

Table 5.4.1: Shared Wildside patterns reused by Corbusier.

| Concern        | Wildside pattern to reuse                                                                                               | Corbusier application                                                                                                              |
| -------------- | ----------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Pagination     | Opaque keyset cursors, envelope responses, hypermedia `next`/`prev` links, and no total-count dependency                | Task lists, project lists, activity feeds, conversations, registries, and audit views                                              |
| Idempotency    | `Idempotency-Key` header, payload hashing, deterministic replay for identical retries, and conflict on payload mismatch | Task creation, task state transitions, conversation sends, directive creation, suggestion accept/dismiss, and governance mutations |
| Data integrity | Revision-aware mutation contracts and explicit conflict payloads                                                        | Project/task edits, suggestion state changes, settings updates, and future collaborative mutations                                 |
| Identity       | Separate tenant identity from user identity, with request context carrying both                                         | Personnel views, settings/auth, audit attribution, task ownership, and future multi-user tenants                                   |
| Streaming      | Explicit SSE event identifiers and replay-aware invalidation semantics                                                  | Dashboard refresh, task activity timelines, conversation updates, tool-run status, and system health screens                       |

Wildside-specific product concepts must not be imported where they do not fit.
Map workflows, offline route bundles, and walk-session models are not part of
this adoption unless a later Corbusier requirement makes them relevant.

### 5.5. Follow the v2a state-management escalation model

The default state model should stay narrow and explicit:

- route-local component state first,
- TanStack Query for server state and cache invalidation,
- Zustand for interactive client state that must outlive a component subtree,
- Dexie only where durable browser-side storage is genuinely required, and
- XState only where the interaction is sufficiently stateful to justify a
  formal machine.

This prevents premature state-framework sprawl while keeping the architecture
compatible with the fuller v2a model already documented in the mockup.

### 5.6. Adopt Nile Valley's same-origin deployment model

The Corbusier PWA should be deployed as a Nile Valley-served web workload that
uses the same origin and platform surface as the backend API.

That implies:

- relative frontend API calls rather than a separate application origin,
- session-aware authentication compatible with backend cookies or the chosen
  Corbusier auth mechanism,
- SSE endpoints reachable without a cross-origin compatibility layer, and
- deployment packaging that follows the same operational model already used for
  Wildside on Nile Valley.

This is the lowest-friction path for auth, caching, ingress, and preview
environments.

### 5.7. Migrate screen by screen rather than with a flag day

The production adoption should proceed in layers:

1. Establish the shell, providers, localization runtime, tokens, and route
   structure in `frontend-pwa/`.
2. Promote fixture-backed mockup screens into the workspace with minimal
   behaviour changes.
3. Replace fixtures with live Corbusier backend projections feature by feature.
4. Add streaming, mutation, and contract-test coverage per feature group.
5. Only add heavier local-first persistence features once the corresponding
   operational need is clear.

This allows backend and frontend work to advance in parallel without forcing a
single cut-over event.

## 6. Requirements

### 6.1. Functional requirements

- The adopted PWA must cover the screen families already proven in
  [`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup):
  dashboard, tasks, projects, conversations, directives, AI suggestions, system
  pages, and settings/global interactions.
- The frontend must consume backend projections that preserve the mockup's card
  and detail-view semantics rather than flattening them into generic transport
  models.
- Mutation flows must support deterministic retries and explicit conflict
  reporting where the backend already requires idempotency or revision checks.
- User identity, tenant identity, and audit attribution must remain distinct in
  both API contracts and frontend presentation.
- The frontend must support live updates for screens whose value depends on
  backend events, particularly dashboard, conversation, task, and system
  monitoring views.

### 6.2. Technical requirements

- The production PWA must use a repository-owned workspace rather than relying
  on [`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup) as
  a runtime dependency.
- The frontend stack must stay aligned with the shared v2a stack unless a later
  RFC or ADR deliberately diverges.
- API list endpoints must follow the same pagination contract across Corbusier
  features so TanStack Query caching and infinite loading remain consistent.
- Mutation endpoints exposed to the PWA must follow a common idempotency and
  optimistic-concurrency contract where those protections apply.
- Localization must keep domain strings in entity models and keep UI chrome in
  Fluent bundles.
- The quality stack must include formatting, linting, strict type-checking,
  component testing, accessibility-focused tests, and end-to-end browser tests.
- The frontend deployment topology must remain compatible with Nile Valley's
  same-origin hosting and ingress assumptions.

## 7. Compatibility and migration

This proposal is intentionally additive and migration-friendly.

[`leynos/corbusier-mockup`](https://github.com/leynos/corbusier-mockup) remains
the proving ground until features are promoted into Corbusier's own
`frontend-pwa/` workspace. No immediate rewrite of existing backend code is
required merely to stand up the workspace shell. The backend changes already
proposed in `docs/corbusier-api-design.md` remain the source of truth for live
data contracts.

Migration should happen in the following order:

1. Accept this RFC and create the `frontend-pwa/` workspace in Corbusier.
2. Promote shared app shell concerns from the mockup: routing, providers,
   design tokens, localization runtime, and baseline test tooling.
3. Land read-only screens first against fixtures or thin adapters so the route
   graph and card composition settle early.
4. Implement backend projections and list/detail endpoints using the shared
   pagination contract.
5. Introduce mutations only after idempotency, revision, and error-envelope
   contracts are present for the affected endpoints.
6. Add SSE-backed invalidation and live updates to the screens that need them.
7. Consider durable browser-side persistence beyond Query cache only where the
   Corbusier use case justifies it.

The migration is compatible with parallel work on backend read models, OpenAPI,
and roadmap tasks. It does not require a one-time switch from mockup to
production UI.

## 8. Alternatives considered

### 8.1. Option A: Treat the mockup as disposable and build a new frontend in Corbusier

This would keep all production code inside one repository from the start, but
it discards the value of the mockup as a proven user-interface and interaction
model. It also invites accidental drift from the validated card schemas and
screen flows already documented. This option was rejected because it pays the
cost of product discovery twice.

### 8.2. Option B: Keep using `leynos/corbusier-mockup` as the effective production frontend

This would minimize short-term migration effort, but it keeps the production
frontend outside the repository that owns the backend contracts, roadmap, and
release workflow. It would make cross-repository versioning, CI, Nile Valley
packaging, and production change control harder than necessary. This option was
rejected because it weakens ownership and makes coupling implicit.

### 8.3. Option C: Copy Wildside's PWA architecture wholesale

Wildside offers strong precedents for cross-cutting concerns, but its product
model is map-centric and offline-bundle-heavy. Corbusier's orchestration
product has different feature priorities and does not need those product
concepts as-is. This option was rejected because it would import unrelated
complexity and obscure the real reuse boundaries.

## 9. Open questions

- Should Corbusier adopt TanStack Query cache persistence immediately, or wait
  until the first live data screens demonstrate a concrete offline or reload
  resilience need?
- Which subset of screens should be promoted first after the app shell:
  dashboard and tasks, or projects and conversations?
- Should the shared pagination, error-envelope, and idempotency contracts live
  in Corbusier-specific packages immediately, or be extracted into a broader
  Nile Valley shared layer only after Corbusier and Wildside have both consumed
  them twice?
- Does Corbusier need service-worker-backed offline behaviour in its first
  production PWA release, or is resilient online-first caching sufficient for
  the initial milestone?

## 10. Recommendation

Adopt the Corbusier frontend PWA by creating a Corbusier-owned `frontend-pwa/`
workspace that imports the proved-out mockup design, preserves the shared v2a
stack and data-model-driven card architecture, and deliberately reuses
Wildside's pagination, idempotency, identity, and SSE patterns where those
concerns are genuinely shared.

This direction captures the validated frontend work already done, keeps the
frontend aligned with Corbusier's backend and roadmap, and fits the Nile Valley
platform model without importing Wildside-specific product concepts that do not
belong in Corbusier.
