# Architectural decision record (ADR) 011: Corbusier / Frankie review adapter role segregation

## Status

Proposed.

## Date

2026-03-28

## Context and Problem Statement

Corbusier's current design positions the platform as the owner of workflow,
task, conversation, tool, and governance state, with Frankie referenced as a
review adapter. In practice, the review design text has still treated Frankie
as a browser-automation-style comment ingester and has folded review handling
into the generic version control system (VCS) boundary.

That no longer matches the intended product split. Frankie is moving toward a
GitHub review adapter and context engine, built around GitHub APIs, local git,
incremental sync, diff-based verification, time-travel context, and reusable
reply tooling.[^1] Corbusier should remain the canonical owner of review
workflow state, because review state must stay correlated with tasks,
conversations, governance outcomes, and tenant-scoped audit records.

This ADR defines the ownership boundary between the two systems, the review
ports Corbusier should expose, and the persistence model Corbusier should own.

## Decision Drivers

- Preserve one canonical workflow owner
- Avoid overloading the generic VCS boundary with review-specialist behaviour
- Keep review-thread state durable, tenant-scoped, and auditable in Corbusier
- Reuse Frankie for GitHub-specific review sync, context, verification, and
  reply execution
- Keep time-travel and verification context reproducible without storing full
  historical snapshots in Corbusier

## Requirements

### Functional requirements

- Corbusier must own canonical review workflow state.
- Corbusier must model review threads as first-class orchestration records.
- Corbusier must support idempotent review sync keyed by pull request and
  thread root.
- Corbusier must preserve structured linkage between review events, tasks, and
  conversation messages.
- Frankie must be able to provide review sync, context materialization,
  verification, and reply execution through a host-friendly library boundary.

### Technical requirements

- The generic VCS adapter contract must remain focused on issue, branch, and
  pull-request lifecycle operations.
- Review integration must use dedicated ports for intake, context, and action
  behaviour.
- Corbusier must persist review threads, review comments, checkpoints, and
  verification results in tenant-scoped tables or equivalent durable
  projections.
- Corbusier must preserve raw Frankie comment payloads losslessly and derive
  anchors only when metadata is sufficient.
- Time-travel context should be materialized on demand inside the task
  workspace rather than stored as full file snapshots in Corbusier.

## Goals and Non-Goals

### Goals

- Make the Corbusier versus Frankie boundary explicit and durable.
- Keep `TaskState` coarse while introducing a sibling review projection for
  thread-specific detail.
- Enable Frankie-backed verification and reply automation without transferring
  workflow ownership away from Corbusier.

### Non-Goals

- Replace the generic VCS adapter with Frankie.
- Make Frankie the system of record for tasks, conversations, or review state.
- Force Corbusier to persist full historical source snapshots for time travel.

## Options Considered

### Option A: Keep review handling inside the generic VCS boundary

Corbusier continues to treat review operations as part of the generic
VCS-provider contract, with Frankie or provider adapters normalizing comments
into a flat review model.

### Option B: Give Corbusier a dedicated review bounded context

Corbusier owns review threads, projections, checkpoints, and linkage to tasks
and conversations. Frankie provides GitHub review sync, time-travel context,
verification, and reply actions behind review-specific ports.

### Option C: Make Frankie the canonical owner of review state

Frankie stores review workflow state and Corbusier consumes it as a downstream
integration feed.

Table 1: Trade-offs for review-role segregation.

| Topic                                     | Option A | Option B | Option C |
| ----------------------------------------- | -------- | -------- | -------- |
| Workflow ownership clarity                | Weak     | Strong   | Weak     |
| Generic VCS boundary cohesion             | Weak     | Strong   | Strong   |
| Corbusier audit and tenancy fit           | Medium   | Strong   | Weak     |
| Frankie reuse for specialist capabilities | Medium   | Strong   | Strong   |
| Integration complexity                    | Medium   | Medium   | High     |

## Decision Outcome / Proposed Direction

Corbusier should introduce a dedicated review bounded context and keep
canonical review workflow state inside Corbusier. Frankie should act as the
GitHub review adapter and context engine.

The boundary should work as follows:

- Generic VCS ports remain responsible for issue, branch, pull-request, and
  webhook lifecycle concerns.
- Review-specific behaviour moves behind `ReviewIntakePort`,
  `ReviewContextPort`, and `ReviewActionPort`.
- Corbusier persists review threads, comments, checkpoints, verification
  results, and message linkage keyed by tenant, pull request, and thread root.
- Corbusier preserves Frankie raw payloads losslessly, then derives
  `ReviewAnchor` data only when commit, file, and line metadata are available.
- `TaskState` remains coarse. A sibling review projection tracks open-thread
  count, verification status, last reviewer action, pending outbound reply, and
  sync checkpoint state.
- Time-travel context is materialized on demand inside the task workspace.
- Verification and reply submission are invoked through Frankie, but the
  resulting workflow state transition is committed by Corbusier.

## Migration Plan

### Phase 1

Define the architectural seam

- [ ] 1.1 Remove review-specific operations from the generic VCS contract in
  the design and roadmap.
- [ ] 1.2 Define `ReviewIntakePort`, `ReviewContextPort`, and
  `ReviewActionPort` in Corbusier-facing design artefacts.

### Phase 2

Persist canonical review state

- [ ] 2.1 Add tenant-scoped review-thread, review-comment, checkpoint, and
  verification persistence models.
- [ ] 2.2 Add message-metadata linkage rules for review-linked conversation
  messages.

### Phase 3

Wire adapter-backed review workflows

- [ ] 3.1 Use Frankie sync checkpoints and thread roots to ingest incremental
  review deltas idempotently.
- [ ] 3.2 Invoke Frankie time-travel context and diff-replay verification from
  Corbusier's governance loop.
- [ ] 3.3 Add reply draft and submission flows that keep Corbusier as the
  canonical owner of the workflow decision.

## Known Risks and Limitations

- Frankie may still need public library-surface work before every desired
  capability is host-consumable.
- Corbusier will need a derived thread-root rule until Frankie exposes a stable
  public thread aggregate.
- Review projections can drift from raw provider state unless sync checkpoints
  and idempotency rules are explicit.
- Reply automation increases the risk of posting against stale state unless
  verification and checkpoint checks run immediately before submission.

## Outstanding Decisions

- What versioned checkpoint envelope Corbusier should persist from Frankie sync
  operations
- Whether Corbusier should queue outbound replies internally before submission
  or submit immediately after policy approval
- Which Frankie reply and time-travel APIs are stable enough to depend on in
  Corbusier's first implementation phase

## Architectural Rationale

This split matches Corbusier's role as the workflow owner and Frankie's role as
the GitHub review adapter plus context engine.[^2] It keeps the architecture
hexagonal: Corbusier owns domain state and orchestration, while Frankie remains
an infrastructure-facing specialist behind ports. The result is a cleaner
generic VCS boundary, durable review workflow state, and a more natural place
to attach tenancy, audit, governance, and conversation linkage.

[^1]: Frankie roadmap and ADRs for incremental review sync and discussion
    contracts:
    [roadmap](https://raw.githubusercontent.com/leynos/frankie/main/docs/roadmap.md),
    [ADR 001](https://raw.githubusercontent.com/leynos/frankie/main/docs/adr-001-incremental-sync-for-review-comments.md),
    and
    [ADR 008](https://raw.githubusercontent.com/leynos/frankie/main/docs/adr-008-pr-discussion-summary-contract.md).
[^2]: Frankie public-surface and reusable capability references:
    [lib.rs](https://raw.githubusercontent.com/leynos/frankie/main/src/lib.rs),
    [GitHub review models](https://raw.githubusercontent.com/leynos/frankie/main/src/github/models/mod.rs),
    [local git integration](https://raw.githubusercontent.com/leynos/frankie/main/src/local/mod.rs),
    and
    [verification module](https://raw.githubusercontent.com/leynos/frankie/main/src/verification/mod.rs).
