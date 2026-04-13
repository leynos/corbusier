# Architectural decision record (ADR) 005: hook execution contract and control-channel semantics

## Status

Accepted.

## Date

2026-03-21

## Context and Problem Statement

The Podbot conformance design[^1] treats hooks as generic execution artefacts
that Podbot runs after Corbusier acknowledges a hook message. That differs
materially from the earlier Corbusier idea of a bespoke in-process hook engine.

This change introduces a new control channel and a new state machine for hosted
sessions. Podbot must be able to emit hook requests, suspend until Corbusier
acknowledges them, execute the hook, and report the resulting lifecycle. That
contract must be explicit, durable, and auditable.

This ADR owns the hook trigger vocabulary, hook request and acknowledgement
messages, and the control-channel semantics. It does not own the persistence
schema for runtime state; ADR 006 covers that.

## Decision Drivers

- Governance and policy enforcement
- Runtime determinism
- Channel separation and protocol purity
- Idempotent recovery after restart
- Clear failure semantics for blocked or denied hooks

## Requirements

### Functional requirements

- The design must define the hook trigger vocabulary.
- The design must define the hook request message schema from Podbot to
  Corbusier.
- The design must define the acknowledgement schema from Corbusier to Podbot.
- The design must define workspace access mode per hook execution.
- The design must define whether completion messages are mandatory, optional,
  or out of scope.

### Technical requirements

- Hook invocation must carry stable correlation identifiers.
- Acknowledgements must be idempotent.
- The runtime must survive restarts without duplicate approvals.
- The design must define timeout behaviour, denial behaviour, and abort
  behaviour explicitly.

## Goals and Non-Goals

### Goals

- Make hook execution a Podbot-owned runtime action governed by Corbusier.
- Preserve agent protocol purity by keeping hook traffic off the agent stream.
- Define deterministic suspend, acknowledge, execute, and complete semantics.

### Non-Goals

- Standardize the contents of all hook artefacts.
- Define every future workflow trigger Corbusier might invent.
- Replace the need for durable state and audit persistence.

## Podbot roadmap dependencies

This ADR is ratified against the following upstream Podbot roadmap steps:

- Step 4.6, "Hosted session control plane" because hook requests and
  acknowledgements must travel over a typed event and control surface.
- Step 4.9, "Hook execution and orchestrator acknowledgement" because that
  step provides the direct Podbot implementation surface for this contract.

The downstream delivery work ratified by this ADR then relies on:

- Step 4.10, "Recovery, replay, and restart safety" because hook
  acknowledgement semantics are incomplete without restart-safe replay and
  duplicate-delivery handling.

## Options Considered

### Option A: Podbot emits requests, waits for acknowledgement, and executes hooks

Podbot sends a typed hook request over a dedicated control channel. Corbusier
evaluates policy and responds with an idempotent acknowledgement that either
approves, denies, or aborts the hook request. Podbot then executes or skips the
hook according to that acknowledgement.

### Option B: Corbusier executes hooks itself

Podbot forwards hook requests to Corbusier, and Corbusier performs the hook
execution directly inside its own runtime boundary.

### Option C: Hooks are advisory and do not suspend execution

Hook requests are emitted for observation, but hosted execution continues
without waiting for Corbusier to decide.

Table 1: Trade-offs for the hook execution contract.

| Topic                       | Option A | Option B | Option C |
| --------------------------- | -------- | -------- | -------- |
| Runtime ownership alignment | Strong   | Weak     | Weak     |
| Governance strength         | Strong   | Strong   | Weak     |
| Protocol purity             | Strong   | Medium   | Medium   |
| Restart safety              | Strong   | Medium   | Weak     |
| Operational clarity         | Strong   | Medium   | Weak     |

## Decision Outcome / Proposed Direction

Corbusier adopts a Podbot-owned hook execution model with a dedicated control
channel and an explicit acknowledgement contract.

The proposed contract is:

- Podbot emits a typed hook request with a stable correlation identifier,
  workspace identifier, trigger name, requested access mode, and artefact
  reference.
- Corbusier evaluates policy and replies with an idempotent acknowledgement
  that explicitly approves, denies, or aborts the request.
- Podbot suspends the relevant hosted execution path until it receives that
  acknowledgement or a timeout occurs.
- Podbot executes the hook only after approval and reports completion or
  terminal failure back over the control channel.
- Completion messages should be treated as mandatory for audit completeness.

## Migration Plan

This ADR lands during ADR 010 Phase 1 (foundational architecture). The
implementation steps below are scoped to this ADR; see ADR 010 for the
cross-cutting migration sequence and advancement criteria.

Roadmap item `1.1.1` accepted this ADR as part of the ADR 001 through 005
bundle. Reviewer-facing compatibility, warn-only, and blocking gates live in
ADR 010 and `docs/podbot-migration-review-checklist.md`; this ADR only scopes
the hook-specific delivery sequence below.

### Phase 1

Define the hook request, acknowledgement, and completion message shapes.

### Phase 2

Add the HookCoordinator in Corbusier and the corresponding Podbot-facing
adapter behaviour for suspend-until-acknowledged flows.

### Phase 3

Remove Corbusier-side assumptions that hooks are executed locally for
Podbot-hosted sessions.

## Known Risks and Limitations

- Denial semantics can become ambiguous unless the acknowledgement contract
  distinguishes skip, fail-current-step, and abort-session outcomes clearly.
- Mandatory completion events increase recovery complexity after runtime
  interruptions.
- Hook requests that depend on wires or other runtime resources require clear
  ordering with other Podbot runtime actions.

## Outstanding Decisions

- Whether denial aborts the session, skips the hook, or fails the current step
- Whether any hook completion event may be optional for specific trigger types
- Whether hooks may depend on MCP wires or other runtime resources
- Whether trigger vocabulary belongs primarily to Corbusier workflow concepts
  or Podbot runtime concepts

## Architectural Rationale

This direction preserves the boundary established by ADR 001 and avoids making
Corbusier a second execution host. Corbusier governs hooks through policy and
acknowledgement. Podbot executes them inside the same runtime boundary that
owns the workspace and hosted session.

[^1]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
