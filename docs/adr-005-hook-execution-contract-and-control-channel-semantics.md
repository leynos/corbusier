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
specifies `approved`, `skip`, `fail-current-step`, or `abort-session` for the
hook request. Podbot then executes or skips the hook according to that
acknowledgement.

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

Trigger vocabulary ownership is decided by the Hook event definitions, which
provide the normative vocabulary for the Podbot-owned hook execution model.

The proposed contract is:

- Podbot emits a typed hook request with a stable correlation identifier,
  workspace identifier, trigger name, requested access mode, and artefact
  reference.
- Corbusier evaluates policy and replies with an idempotent acknowledgement
  that specifies `approved`, `skip`, `fail-current-step`, or `abort-session` for
  the request.
- Podbot suspends the relevant hosted execution path until it receives that
  acknowledgement or a timeout occurs.
- Podbot executes the hook only after approval and reports completion or
  terminal failure back over the control channel.
- Completion messages should be treated as mandatory for audit completeness.

### Hook event definitions

Each hook event carries a stable trigger name, a typed payload, and a
well-defined acknowledgement contract. The trigger vocabulary is defined below
with exact payload schemas, acknowledgement semantics, and timeout and resume
rules.

#### PreTurn

- **Trigger name:** `pre-turn`
- **Payload fields:**
  - `correlationId` (string, UUID): stable identifier for this hook invocation
  - `workspaceId` (string): identifier of the hosted workspace
  - `triggerName` (string): `"pre-turn"`
  - `accessMode` (string): `"ro"` (read-only workspace access)
  - `turnIndex` (number): the index of the upcoming turn within the session
- **Acknowledgement semantics:** Podbot suspends the hosted execution path
  until it receives an idempotent `approved`, `skip`, `fail-current-step`, or `abort-session`
  acknowledgement. Corbusier must respond within the configured timeout.
- **Timeout and resume rules:** timeout defaults to 30 seconds. On timeout,
  Podbot treats the unacknowledged hook as `abort-session` and terminates the
  session.

#### PostTurn

- **Trigger name:** `post-turn`
- **Payload fields:**
  - `correlationId` (string, UUID): stable identifier for this hook invocation
  - `workspaceId` (string): identifier of the hosted workspace
  - `triggerName` (string): `"post-turn"`
  - `accessMode` (string): `"ro"` (read-only workspace access)
  - `turnIndex` (number): the index of the completed turn
  - `artefactRef` (string, optional): reference to the turn result artefact
- **Acknowledgement semantics:** Podbot suspends the hosted execution path
  until it receives an idempotent `approved`, `skip`, `fail-current-step`, or `abort-session`
  acknowledgement. Corbusier must respond within the configured timeout.
- **Timeout and resume rules:** timeout defaults to 30 seconds. On timeout,
  Podbot treats the unacknowledged hook as `abort-session` and terminates the
  session.

#### PreToolCall

- **Trigger name:** `pre-tool-call`
- **Payload fields:**
  - `correlationId` (string, UUID): stable identifier for this hook invocation
  - `workspaceId` (string): identifier of the hosted workspace
  - `triggerName` (string): `"pre-tool-call"`
  - `accessMode` (string): `"ro"` (read-only workspace access)
  - `toolName` (string): name of the tool about to be invoked
  - `toolInput` (object): the input parameters for the tool call
- **Acknowledgement semantics:** Podbot suspends the hosted execution path
  until it receives an idempotent `approved`, `skip`, `fail-current-step`, or `abort-session`
  acknowledgement. Corbusier must respond within the configured timeout. A
  `skip` denial blocks the tool call; a `fail-current-step` denial blocks the
  tool call and marks the current step failed; an `abort-session` denial
  terminates the session.
- **Timeout and resume rules:** timeout defaults to 15 seconds. On timeout,
  Podbot treats the unacknowledged hook as `skip` (deny the tool call without
  failing the step).

#### PostToolCall

- **Trigger name:** `post-tool-call`
- **Payload fields:**
  - `correlationId` (string, UUID): stable identifier for this hook invocation
  - `workspaceId` (string): identifier of the hosted workspace
  - `triggerName` (string): `"post-tool-call"`
  - `accessMode` (string): `"ro"` (read-only workspace access)
  - `toolName` (string): name of the tool that was invoked
  - `toolOutput` (object): the output from the tool call
- **Acknowledgement semantics:** Podbot does not suspend execution for
  `post-tool-call` hooks. The acknowledgement may be deferred or omitted; if
  omitted, Podbot treats it as `approved` by default.
- **Timeout and resume rules:** no timeout is enforced for `post-tool-call`
  hooks. Podbot continues execution immediately after emitting the hook
  request.

#### PreCommit

- **Trigger name:** `pre-commit`
- **Payload fields:**
  - `correlationId` (string, UUID): stable identifier for this hook invocation
  - `workspaceId` (string): identifier of the hosted workspace
  - `triggerName` (string): `"pre-commit"`
  - `accessMode` (string): `"rw"` (read-write workspace access)
  - `commitMessage` (string): the proposed commit message
  - `changedFiles` (array of string): paths of files included in the commit
  - `diff` (string, optional): the full diff of the proposed commit
- **Acknowledgement semantics:** Podbot suspends the commit operation until it
  receives an idempotent `approved`, `skip`, `fail-current-step`, or
  `abort-session` acknowledgement. Corbusier must respond within the
  configured timeout.
- **Timeout and resume rules:** timeout defaults to 60 seconds. On timeout,
  Podbot treats the unacknowledged hook as `skip` (block the commit without
  failing the session).

#### PreMerge

- **Trigger name:** `pre-merge`
- **Payload fields:**
  - `correlationId` (string, UUID): stable identifier for this hook invocation
  - `workspaceId` (string): identifier of the hosted workspace
  - `triggerName` (string): `"pre-merge"`
  - `accessMode` (string): `"rw"` (read-write workspace access)
  - `sourceBranch` (string): the branch being merged
  - `targetBranch` (string): the target branch
- **Acknowledgement semantics:** Podbot suspends the merge operation until it
  receives an idempotent `approved`, `skip`, `fail-current-step`, or
  `abort-session` acknowledgement. Corbusier must respond within the
  configured timeout.
- **Timeout and resume rules:** timeout defaults to 120 seconds. On timeout,
  Podbot treats the unacknowledged hook as `skip` (block the merge without
  failing the session).

#### PreDeploy

- **Trigger name:** `pre-deploy`
- **Payload fields:**
  - `correlationId` (string, UUID): stable identifier for this hook invocation
  - `workspaceId` (string): identifier of the hosted workspace
  - `triggerName` (string): `"pre-deploy"`
  - `accessMode` (string): `"rw"` (read-write workspace access)
  - `target` (string): the deployment target identifier
  - `artefactRef` (string): reference to the build or image being deployed
- **Acknowledgement semantics:** Podbot suspends the deployment operation
  until it receives an idempotent `approved`, `skip`, `fail-current-step`, or
  `abort-session` acknowledgement. Corbusier must respond within the
  configured timeout.
- **Timeout and resume rules:** timeout defaults to 300 seconds. On timeout,
  Podbot treats the unacknowledged hook as `abort-session` and terminates the
  session.

## Migration Plan

This ADR lands during ADR 010 Phase 1 (foundational architecture). The
implementation steps below are scoped to this ADR; see ADR 010 for the
cross-cutting migration sequence and advancement criteria.

### Hook denial outcomes

Corbusier's idempotent acknowledgement must distinguish three denial outcomes
with precise downstream semantics so that Podbot and Corbusier implementers
share an unambiguous contract:

- **`skip`** — The hook is not executed. Podbot must continue to the next hook
  in the delivery sequence without marking the current step or session as
  failed. The hook delivery sequence resumes at the next scheduled trigger for
  the same session. Downstream components must treat a `skip` denial as
  non-terminal: the session remains active and the step that triggered the
  hook remains unaffected.
- **`fail-current-step`** — The hook is not executed. Podbot must mark the
  current logical step as failed but continue the session. Subsequent hooks
  whose trigger depends on a step that is now failed must be suppressed; hooks
  that do not depend on the failed step continue normally. Downstream
  components must record the step failure in the audit trail and propagate the
  failure status to any consumer of the step result.
- **`abort-session`** — The hook is not executed. Podbot must terminate the
  session immediately, discard any pending hook requests, and transition the
  hosted session to a terminal state. No further hook delivery occurs for this
  session. Downstream components must release all resources associated with
  the session and finalize the audit trail.

These denial outcomes map to the REVIEW gates defined in ADR 010 and
`docs/podbot-migration-review-checklist.md`: a `skip` denial is a warn-only
gate for the affected hook; a `fail-current-step` denial is a warn-only gate
for the session but a blocking gate for the step; an `abort-session` denial is
a blocking gate for the session.

Roadmap item `1.1.1` accepted this ADR as part of the ADR 001–005 and ADR 010
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

## Architectural Rationale

This direction preserves the boundary established by ADR 001 and avoids making
Corbusier a second execution host. Corbusier governs hooks through policy and
acknowledgement. Podbot executes them inside the same runtime boundary that
owns the workspace and hosted session.

[^1]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
