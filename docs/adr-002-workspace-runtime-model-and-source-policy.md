# Architectural decision record (ADR) 002: workspace runtime model and source policy

## Status

Accepted.

## Date

2026-03-21

## Context and Problem Statement

ADR 001 proposes that Podbot owns runtime mechanics for hosted sessions.
Corbusier still needs a stable workspace model because prompt validation, MCP
wire provisioning, hook execution, and audit correlation all depend on a shared
vocabulary for workspace identity and lifecycle.

Corbusier must distinguish between the logical workspace record that it uses
for orchestration and the concrete runtime workspace that Podbot prepares. The
design must also define the allowed workspace sources, access modes, retention
behaviour, and safety checks for host mounts.

This ADR owns workspace identity, lifecycle, and mount policy. It does not own
wire semantics, hook semantics, or durable audit state beyond the workspace
record itself.

## Decision Drivers

- Reproducibility of runtime state
- Least privilege by default
- Deterministic cleanup and garbage collection
- Clear correlation between task identity and workspace identity
- Restart safety and operational debuggability

## Requirements

### Functional requirements

- A workspace must have a stable identifier.
- A workspace must bind cleanly to task, session, and tenant context.
- The system must support explicit workspace source types.
- The system must support explicit access modes for hosted agents and hooks.
- The system must define lifecycle events such as prepared, active, drained,
  failed, and cleaned up.

### Technical requirements

- Host mounts must pass canonicalization and allowed-root checks.
- The design must reject or neutralize symlink escape paths.
- Read-only versus read-write access must be explicit, auditable, and not
  implied by caller behaviour.
- Workspace cleanup must be deterministic and idempotent.

## Goals and Non-Goals

### Goals

- Give Corbusier one canonical workspace runtime record.
- Support more than one workspace source without leaving behaviour implicit.
- Make access mode and cleanup policy explicit and auditable.

### Non-Goals

- Define the detailed schema for wire records or hook records.
- Define tenant retention policy for all audit artefacts.
- Expose host paths directly to agents as stable identifiers.

## Podbot roadmap dependencies

This ADR is ratified against the following upstream Podbot roadmap steps:

- Step 1.4, "Hosting schema migration and compatibility matrix" because
  workspace source and mount-related configuration must exist in Podbot's typed
  hosting schema.
- Step 4.4, "Workspace strategies" because that step delivers the clone and
  host-mount runtime behaviours that this ADR relies on.

The downstream delivery work ratified by this ADR then relies on:

- Step 4.5, "Normalized launch contract" because workspace source, mount
  policy, and access mode should be resolved through one normalized launch path.

## Options Considered

### Option A: Corbusier owns the logical record and Podbot owns the concrete runtime state

Corbusier persists a workspace runtime record with identity, source metadata,
requested access mode, and lifecycle state. Podbot prepares and tears down the
concrete runtime realization for that record.

### Option B: Corbusier owns both logical and concrete workspace state

Corbusier persists and directly manages the prepared workspace, while Podbot
only receives enough information to start the container.

### Option C: Podbot owns the entire workspace model

Corbusier stores only opaque workspace identifiers returned by Podbot and
relies on Podbot to remain the sole source of truth for all workspace detail.

Table 1: Trade-offs for the workspace runtime model.

| Topic                    | Option A | Option B | Option C |
| ------------------------ | -------- | -------- | -------- |
| Corbusier auditability   | Strong   | Strong   | Weak     |
| Runtime duplication risk | Low      | High     | Low      |
| Restart recovery clarity | Strong   | Medium   | Weak     |
| Podbot coupling          | Medium   | Low      | High     |
| Operator debuggability   | Strong   | Medium   | Weak     |

## Decision Outcome / Proposed Direction

Corbusier keeps one canonical workspace runtime record while Podbot owns the
concrete prepared workspace for hosted execution.

The proposed workspace model is:

- Corbusier creates a stable workspace identifier linked to tenant, task, and
  hosted-session context.
- Corbusier records the requested source type, requested access mode, and
  lifecycle state.
- Podbot prepares the concrete runtime workspace for that record and returns
  runtime details needed for later control operations.
- The initial production default should be repository clone into a Podbot-owned
  runtime workspace. Host mounts remain supported, but only through explicit
  policy and safety checks.
- Host mounts must use canonical paths, allowed-root validation, and symlink
  escape protection before Podbot is allowed to mount them.

## Migration Plan

This ADR lands during ADR 010 Phase 1 (foundational architecture). The
implementation steps below are scoped to this ADR; see ADR 010 for the
cross-cutting migration sequence and advancement criteria.

Roadmap item `1.1.1` accepted this ADR as part of the ADR 001 through 005
bundle. Reviewer-facing compatibility, warn-only, and blocking gates live in
ADR 010 and `docs/podbot-migration-review-checklist.md`; this ADR only scopes
the workspace-specific delivery sequence below.

### Phase 1

Introduce the Corbusier workspace runtime record and lifecycle state machine.

### Phase 2

Teach the Podbot adapter to prepare clone-backed workspaces and report concrete
runtime identifiers back to Corbusier.

### Phase 3

Add host-mount support with canonicalization, allowed-root enforcement, and
explicit read-only or read-write access controls.

## Known Risks and Limitations

- Repository clones and host mounts may diverge in cleanup timing and failure
  semantics.
- Debug retention for failed workspaces can delay cleanup and consume storage.
- Hooks that need wider access than the hosted agent can create pressure to
  widen permissions unless the policy model is explicit.

## Outstanding Decisions

- Whether repository clone and host mount share identical lifecycle semantics
- Whether hooks can ever widen access from read-only to read-write
- How long failed workspaces are retained for debugging before cleanup
- Whether cleanup may run before all audit artefacts are persisted

## Architectural Rationale

This direction keeps the Corbusier record rich enough for orchestration,
restart recovery, and audit, while avoiding a second runtime implementation. It
also gives later ADRs a stable unit of attachment for wires, hooks, and prompt
validation. The design boundaries in this ADR trace back to the companion
design[^1].

[^1]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
