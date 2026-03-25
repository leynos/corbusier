# Architectural decision record (ADR) 010: migration and coexistence strategy

## Status

Proposed.

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
- Minimised operational risk
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

_Table 1: Trade-offs for the migration strategy._

| Topic                  | Option A | Option B | Option C |
| ---------------------- | -------- | -------- | -------- |
| Delivery risk          | Low      | High     | Medium   |
| Reviewer clarity       | Strong   | Weak     | Weak     |
| Migration duration     | Medium   | Low      | High     |
| Technical debt control | Strong   | Medium   | Weak     |
| Rollback safety        | Strong   | Weak     | Medium   |

## Decision Outcome / Proposed Direction

Corbusier should adopt a staged migration with compatibility adapters, warning
phases, explicit advancement criteria, and hard removal criteria for legacy
assumptions.

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

Advancement between phases should require quality gates, migration tests, and
review confirmation that the preceding ADR-owned boundary is no longer being
re-litigated in implementation pull requests.

## Migration Plan

### Phase 1

Foundational architecture

- [ ] 1.1 Ratify ADRs 001 through 005. Finish criteria: ADRs 001 through 005
  remain `Proposed` or advance together with no contradictory dependency text
  and all referenced companion links resolve in CI preview. Dependencies: None.
- [ ] 1.2 Add Podbot-facing adapters and compatibility seams. Finish criteria:
  the Podbot-facing adapter interfaces compile behind the selected feature
  gate, and one integration fixture exercises the adapter boundary without
  reviving inline runtime ownership. Dependencies: 1.1.

### Phase 2

Durability and document surfaces

- [ ] 2.1 Land ADRs 006 through 008 with backing schemas, parsers, and
  fixtures. Finish criteria: runtime-state schema migrations apply cleanly in
  fresh and upgrade paths, and fixtures cover workspace, wire, hook, and
  validation records across at least 3 representative scenarios. Dependencies:
  1.1, 1.2.
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
  historical records. Dependencies: 2.1, 3.1.
- [ ] 3.3 Promote warnings to blocking checks according to defined gates.
  Finish criteria: blocking gates fail closed in CI for all deny-path fixtures
  and pass in 3 consecutive full acceptance runs with no manual bypasses.
  Dependencies: 2.2, 3.1, 3.2.

## Known Risks and Limitations

- Compatibility layers can linger unless removal criteria are explicit and
  enforced.
- Warn-only phases can normalise degraded behaviour if they last too long.
- Rollback paths can become fragile if schema changes are coupled too tightly
  to runtime changes.

## Outstanding Decisions

- How long legacy transport parsing remains supported
- When in-memory or legacy runtime adapters can be deleted
- Which phases require accepted ADRs before merge
- Which roadmap and documentation updates gate the end of migration

## Architectural Rationale

The companion design[^cd] is too broad to deliver safely as one cutover. A
staged migration respects the repository's existing architecture, keeps the
review surface comprehensible, and gives maintainers a disciplined way to
retire old assumptions once the Podbot-conformant path is proven.

[^cd]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
