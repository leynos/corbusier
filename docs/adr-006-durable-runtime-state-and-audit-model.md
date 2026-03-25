# Architectural decision record (ADR) 006: durable runtime state and audit model

## Status

Proposed.

## Date

2026-03-21.

## Context and Problem Statement

ADRs 002 through 005 introduce workspace runtime records, workspace-scoped wire
records, and asynchronous hook coordination. Once Corbusier stops acting as an
inline runtime host and instead consumes runtime events from Podbot, durable
state becomes part of the architecture rather than an implementation detail.

The repository needs a clear answer to what Corbusier persists, how runtime
entities are correlated, and what recovery guarantees exist after process
restart or partial failure.

This ADR owns the durable runtime model for workspaces, wires, hooks, and
hosted sessions, along with the audit correlation rules. It does not redefine
the hook contract or wire model already owned by earlier ADRs.

## Decision Drivers

- Recovery without duplicate side effects
- Audit completeness
- Debuggability of distributed runtime failures
- Data retention cost and operational complexity
- Need for deterministic garbage collection

## Requirements

### Functional requirements

- Corbusier must persist hosted session identity and state.
- Corbusier must persist workspace identity and lifecycle state.
- Corbusier must persist workspace wire records.
- Corbusier must persist hook invocation and acknowledgement state.
- Corbusier must retain sufficient linkage to prompts, bundles, tasks, and
  tenants.

### Technical requirements

- Each entity must have an explicit state model.
- The design must define idempotent transitions.
- The design must define log and artefact references separately from primary
  state rows.
- The design must define cleanup policy and retention policy.

## Goals and Non-Goals

### Goals

- Provide restart-safe runtime orchestration.
- Capture enough history to explain runtime outcomes to operators and reviewers.
- Separate primary state from large logs and artefacts.

### Non-Goals

- Adopt full event sourcing by default.
- Persist every telemetry event forever.
- Replace operational observability tooling with relational state alone.

## Podbot roadmap dependencies

This ADR depends on the following upstream Podbot roadmap steps:

- Step 4.6, "Hosted session control plane" because Corbusier's durable model
  depends on the event boundaries exposed by Podbot's hosted session surface.
- Step 4.9, "Hook execution and orchestrator acknowledgement" because hook
  state and acknowledgements are part of the runtime entities that Corbusier
  must persist.
- Step 4.10, "Recovery, replay, and restart safety" because Podbot's event
  identifiers and recovery behaviour drive Corbusier's idempotency and restart
  guarantees.

## Options Considered

### Option A: Normalized relational runtime state with explicit state machines

Corbusier persists runtime entities in relational tables with explicit states,
correlation identifiers, transition rules, and separate references to logs or
artefacts.

### Option B: Mostly ephemeral runtime state with best-effort logging

Corbusier keeps only a minimal runtime footprint and relies on logs and
telemetry for reconstruction when failures occur.

### Option C: Event-sourced runtime state with read projections

Corbusier records runtime changes only as events and builds projections for
operator views and reconciliation workflows.

| Topic                     | Option A | Option B | Option C |
| ------------------------- | -------- | -------- | -------- |
| Restart recovery          | Strong   | Weak     | Strong   |
| Operational complexity    | Medium   | Low      | High     |
| Audit completeness        | Strong   | Weak     | Strong   |
| Incremental migration fit | Strong   | Medium   | Weak     |
| Query simplicity          | Strong   | Medium   | Weak     |

_Table 1: Trade-offs for durable runtime state._

## Decision Outcome / Proposed Direction

Corbusier should adopt a durable relational runtime model with explicit state
machines and correlation identifiers.

The proposed runtime entities are:

- hosted sessions,
- workspaces,
- workspace wires,
- hook invocations and acknowledgements, and
- validation snapshots or references when needed for audit.

Each entity should expose explicit terminal and non-terminal states, idempotent
transitions, and stable correlation identifiers that connect runtime records to
tasks, prompts, bundles, and tenants. Large logs, transcripts, and artefacts
should remain out of the primary runtime tables and be linked by reference.

## Migration Plan

This ADR lands during ADR 010 Phase 2 (durability and document surfaces). The
implementation steps below are scoped to this ADR; see ADR 010 for the
cross-cutting migration sequence and advancement criteria.

### Phase 1

Define the runtime entities, identifiers, and state machines in Corbusier's
domain and persistence layers.

### Phase 2

Persist new hosted-session, workspace, wire, and hook flows alongside existing
behaviour, with reconciliation logic for restart recovery.

### Phase 3

Move cleanup and retention jobs onto the new runtime entities and remove
remaining best-effort-only runtime paths.

## Known Risks and Limitations

- Runtime tables can grow quickly if retention and cleanup policies are not
  bounded.
- Tool-call telemetry may arrive out of order relative to state transitions and
  require reconciliation.
- Over-normalisation can slow down operator queries if read models are not
  designed carefully.

## Outstanding Decisions

- Whether direct tool-call telemetry arrives as raw events or derived audit
  logs
- Whether retention differs by tenant or runtime surface
- Which failure states are terminal versus reconcilable
- How cleanup jobs avoid deleting data still needed by review tooling

## Architectural Rationale

The companion design[^cd] needs Corbusier to remain the durable authority even
while Podbot owns runtime execution. A relational state model with explicit
state machines matches that role, supports incremental migration, and is easier
to review and operate than either best-effort logging or full event sourcing.

[^cd]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
