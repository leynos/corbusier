# Architectural decision record (ADR) 010: migration and coexistence strategy

## Status

Accepted.

## Date

2026-03-21

## Context and Problem Statement

Corbusier already contains parts of the outgoing architecture, including the
current MCP server registry, tool routing assumptions, and earlier hook-engine
concepts. ADRs 001 through 009 define the proposed Podbot-conformant target,
but the repository still needs a controlled way to move from the current state
to that target without a flag day.

This ADR owns the migration phases, compatibility windows, deprecation gates,
rollback expectations, and coexistence rules for the transition.

## Decision Drivers

- Continuous repository health during migration
- Minimized operational risk
- Predictable deprecation of legacy terms and code paths
- Controlled introduction of new schemas and services
- Clear reviewer checkpoints

## Requirements

### Functional requirements

- The design must define migration phases.
- The design must define compatibility behaviour for legacy transport records.
- The design must define when warn-only validation becomes blocking
  validation.
- The design must define when direct tool-routing assumptions are removed for
  Podbot-hosted agents.

### Technical requirements

- The design must define schema migration order.
- The design must define the minimum automated test coverage needed before each
  phase advances.
- The design must define feature flags or equivalent rollout guards, if any.
- The design must define rollback and recovery expectations.

## Goals and Non-Goals

### Goals

- Deliver the new architecture in staged, reviewable increments.
- Keep the repository healthy while old and new models temporarily coexist.
- Provide explicit removal criteria for legacy assumptions.

### Non-Goals

- Preserve the old and new architectures indefinitely.
- Require one large cutover pull request.
- Freeze all unrelated repository work until migration finishes.

## Podbot roadmap dependencies

This ADR depends on the following upstream Podbot roadmap steps because they
define the external surfaces that Corbusier must migrate against:

- Step 1.4, "Hosting schema migration and compatibility matrix"
- Step 4.5, "Normalized launch contract"
- Step 4.6, "Hosted session control plane"
- Step 4.7, "MCP wire provisioning and injection"
- Step 4.8, "Prompt, bundle, and validation surfaces"
- Step 4.9, "Hook execution and orchestrator acknowledgement"
- Step 4.10, "Recovery, replay, and restart safety"

## Options Considered

### Option A: Staged migration with compatibility adapters and phase gates

Corbusier introduces the new runtime, wire, hook, validation, and document
surfaces incrementally behind explicit phases, compatibility adapters, and
removal criteria.

### Option B: Big-bang cutover

Corbusier replaces the old architecture with the new architecture in one large
transition.

### Option C: Permanent dual architecture

Corbusier keeps the old and new architectural paths side by side with no hard
deprecation point.

Table 1: Trade-offs for the migration strategy.

| Topic                  | Option A | Option B | Option C |
| ---------------------- | -------- | -------- | -------- |
| Delivery risk          | Low      | High     | Medium   |
| Reviewer clarity       | Strong   | Weak     | Weak     |
| Migration duration     | Medium   | Low      | High     |
| Technical debt control | Strong   | Medium   | Weak     |
| Rollback safety        | Strong   | Weak     | Medium   |

## Decision Outcome / Proposed Direction

Corbusier adopts a staged migration with compatibility adapters, warn-only
diagnostics, explicit advancement criteria, and hard removal criteria for
legacy assumptions.

The proposed migration sequence is:

1. Land the foundational ADRs and the Podbot-facing runtime boundary.
2. Introduce workspace runtime records, wire records, and the canonical source
   taxonomy with compatibility parsing for legacy data.
3. Add the hook control channel, durable runtime state, and prompt and bundle
   document model.
4. Introduce structured prompt validation and privilege-default enforcement in
   warn-only mode where needed.
5. Remove direct tool-routing assumptions, retire legacy transport labels from
   normal write paths, and tighten validation and privilege checks to blocking
   behaviour.

Roadmap item `1.1.1` accepts ADRs 001 through 005 together with this ADR as one
foundational bundle. No pull request may claim compatibility, warn-only, or
blocking status unless that accepted bundle remains intact and the reviewer
evidence in `docs/podbot-migration-review-checklist.md` is attached.

Migration stages are repository governance states, not permission to split
runtime ownership:

### Compatibility stage

- Entry criteria: ADRs 001 through 005 and ADR 010 are accepted; this ADR, the
  roadmap, the design document, and the migration checklist all describe the
  same Podbot-owned runtime boundary; and the pull request routes new hosted
  work through a Podbot-facing seam or an explicit compatibility adapter
  without reviving inline runtime ownership in Corbusier.
- Exit criteria: canonical write paths exist for the in-scope surface;
  compatibility reads for retained history are proven where required; and the
  remaining legacy paths are named, scoped, and assigned to later roadmap items.
- Rollback expectation: revert to the previous compatibility adapter or freeze
  the new write path, but do not reintroduce inline runtime ownership,
  Corbusier-side hosted tool routing, or Corbusier-side hook execution for
  Podbot-hosted sessions.

### Warn-only stage

- Entry criteria: the affected surface has already met the compatibility-stage
  exit criteria; diagnostics are persisted or otherwise reviewable; and warning
  evaluation blocks zero supported flows by itself.
- Exit criteria: reviewed fixtures and sampled production-like inputs show that
  warnings align with the intended blocking policy, and every remaining warning
  class has a documented owner in the roadmap or checklist evidence.
- Rollback expectation: return to compatibility-stage behaviour while
  preserving the warning instrumentation and the canonical model introduced in
  the compatibility stage.

### Blocking stage

- Entry criteria: the affected surface has met the warn-only exit criteria; CI
  deny-path fixtures fail closed with no manual bypass in the evidence bundle;
  rollback steps are documented; and any compatibility reads retained by ADR
  004 remain available for historical records.
- Exit criteria: roadmap item `1.6.2` is complete, legacy removals no longer
  need compatibility reads or warn-only diagnostics for the retired surface,
  and the documentation gates below have all been met.
- Rollback expectation: fall back only to warn-only behaviour, record the
  regression in the checklist evidence, and keep the accepted runtime boundary
  intact.

Migration cannot be declared complete until `docs/roadmap.md`,
`docs/corbusier-design.md`, this ADR, and
`docs/podbot-migration-review-checklist.md` all point at the same hosted path.
`docs/users-guide.md` changes are required only if a migration gate introduces
operator-visible behaviour that users must follow.

## Migration Plan

Use `docs/podbot-migration-review-checklist.md` as the operational companion
for the gates below.

### Phase 1

Foundational architecture

- [ ] 1.1 Ratify ADRs 001 through 005. Finish criteria: ADRs 001 through 005
  and ADR 010 are `Accepted`, no contradictory dependency or ownership text
  remains, the migration checklist exists, and all referenced companion links
  resolve in CI preview. Dependencies: None.
- [ ] 1.2 Add Podbot-facing adapters and compatibility seams. Finish criteria:
  the Podbot-facing adapter interfaces compile behind the selected feature
  gate, and one integration fixture exercises the adapter boundary without
  reviving inline runtime ownership. Dependencies: 1.1.

### Phase 2

Durability and document surfaces

- [ ] 2.1 Land ADRs 006 through 008 with backing schemas, parsers, and
  fixtures. Finish criteria: runtime-state schema migrations apply cleanly in
  fresh and upgrade paths, and fixtures cover workspace, wire, hook, and
  validation records across at least 3 representative scenarios. This opens the
  compatibility stage for those surfaces only when the checklist evidence is
  present. Dependencies: 1.1, 1.2.
- [ ] 2.2 Run warn-only validation where blocking behaviour would break active
  flows. Finish criteria: warn-only validation runs against at least 10
  reviewed prompt samples, records diagnostics for each sample, and blocks 0
  production paths solely due to validation. Dependencies: 2.1.

### Phase 3

Security tightening and retirement

- [ ] 3.1 Land ADR 009 defaults and override controls. Finish criteria:
  privilege defaults, override records, and approval hooks are persisted and
  surfaced in operator review flows for 100% of override requests in the
  acceptance suite. Dependencies: 2.1.
- [ ] 3.2 Remove legacy routing and legacy transport write paths. Finish
  criteria: no new writes use legacy routing or legacy transport labels, and
  the migration suite proves compatibility reads still succeed for retained
  historical records. This closes the compatibility stage for those legacy
  write paths. Dependencies: 2.1, 3.1.
- [ ] 3.3 Promote warnings to blocking checks according to defined gates.
  Finish criteria: blocking gates fail closed in CI for all deny-path fixtures
  and pass in 3 consecutive full acceptance runs with no manual bypasses. This
  opens the blocking stage only after the warn-only checklist exit criteria are
  satisfied. Dependencies: 2.2, 3.1, 3.2.

## Known Risks and Limitations

- Compatibility layers can linger unless removal criteria are explicit and
  enforced.
- Warn-only phases can normalize degraded behaviour if they last too long.
- Rollback paths can become fragile if schema changes are coupled too tightly
  to runtime changes.

## Outstanding Decisions

- How long legacy transport parsing remains supported
- When in-memory or legacy runtime adapters can be deleted

## Architectural Rationale

The companion design[^1] is too broad to deliver safely as one cutover. A
staged migration respects the repository's existing architecture, keeps the
review surface comprehensible, and gives maintainers a disciplined way to
retire old assumptions once the Podbot-conformant path is proven.

[^1]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
