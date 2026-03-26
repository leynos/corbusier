# Architectural decision record (ADR) 009: security and privilege boundary defaults

## Status

Proposed.

## Date

2026-03-21

## Context and Problem Statement

The Podbot-conformant architecture introduces several security-sensitive
surfaces at once: workspace mounts, helper-container repository access, hook
execution, secret injection, prompt-selected wires, and host-enforced
capabilities. Without one authoritative default privilege stance, these
surfaces will drift independently and become difficult to review.

This ADR owns the default privilege model and the override rules for hosted
agents, hooks, helper-container sources, and delegated host capabilities. It
does not redefine the underlying workspace, wire, or hook models.

## Decision Drivers

- Least privilege by default
- Secret containment
- Tenant isolation
- Operator reviewability of overrides
- Reduced blast radius for compromised tools or prompts

## Requirements

### Functional requirements

- The design must define default access levels for hosted agents, hooks, and
  helper-container sources.
- The design must define how overrides are requested, approved, and audited.
- The design must define how secrets are injected and redacted.
- The design must define unsafe combinations that the runtime rejects outright.

### Technical requirements

- The design must treat environment allowlists as enforcement, not annotation.
- The design must define how repository access is represented in persisted
  source definitions.
- The design must define how security-relevant override use is surfaced in
  audit records.
- The design must define whether privileged overrides are tenant-scoped,
  session-scoped, or both.

## Goals and Non-Goals

### Goals

- Default to the narrowest permissions that still permit useful hosted work.
- Make every privilege increase explicit, reviewable, and auditable.
- Reject unsafe combinations by design rather than by convention.

### Non-Goals

- Eliminate all privileged operations in every environment.
- Encode organisation-specific approval processes in full detail.
- Replace host operating-system safeguards with application policy alone.

## Podbot roadmap dependencies

This ADR depends on the following upstream Podbot roadmap steps:

- Step 1.4, "Hosting schema migration and compatibility matrix" because the
  security-sensitive hosting fields and defaults must exist in Podbot's
  configuration model.
- Step 2.6, "ACP capability masking enforcement" because delegated host
  capability rules are part of the security boundary this ADR sets.
- Step 4.4, "Workspace strategies" because host-mount policy and allowed-root
  enforcement are core privilege-boundary controls.
- Step 4.7, "MCP wire provisioning and injection" because helper-container
  `RepoAccess` defaults and wire injection details affect the runtime trust
  boundary.
- Step 4.9, "Hook execution and orchestrator acknowledgement" because hook
  workspace access and environment allowlist policy are security-critical
  defaults.

## Options Considered

### Option A: Least-privilege defaults with explicit, auditable opt-in

Hosted agents, hooks, and helper containers start with narrow access defaults.
Broader access requires explicit policy, approval, and audit capture.

### Option B: Permissive defaults with selective deny rules

Most capabilities are enabled by default, and only a short denylist blocks the
most dangerous combinations.

### Option C: Environment-specific defaults

Development uses permissive defaults while production uses stricter ones, with
the effective privilege stance changing by deployment environment.

Table 1: Trade-offs for privilege defaults and overrides.

| Topic                      | Option A | Option B | Option C |
| -------------------------- | -------- | -------- | -------- |
| Security posture           | Strong   | Weak     | Medium   |
| Reviewability              | Strong   | Medium   | Weak     |
| Operator surprise          | Low      | Medium   | High     |
| Blast radius               | Low      | High     | Medium   |
| Implementation convenience | Medium   | High     | Medium   |

## Decision Outcome / Proposed Direction

Corbusier should adopt least-privilege defaults with explicit, reviewable, and
auditable overrides.

The proposed defaults are:

- hosted agents start with the narrowest workspace access required by the
  chosen task and prompt,
- hooks declare an explicit access mode and do not inherit broader access by
  default,
- helper-container sources default to no repository access unless policy grants
  read-only or read-write access explicitly, and
- delegated host capabilities are disabled unless the target runtime and policy
  both opt in.

Environment allowlists must be enforced at runtime. Unsafe privilege
combinations should be rejected by policy and runtime validation rather than
merely documented as discouraged.

## Migration Plan

This ADR lands during ADR 010 Phase 3 (security tightening and retirement). The
implementation steps below are scoped to this ADR; see ADR 010 for the
cross-cutting migration sequence and advancement criteria.

### Phase 1

Define the default privilege matrix for hosted agents, hooks, helper
containers, and delegated host capabilities.

### Phase 2

Add explicit override records, approval hooks, and audit capture for override
use.

### Phase 3

Block previously implicit or permissive paths once the new policy and review
surfaces are in place.

## Known Risks and Limitations

- Least-privilege defaults can create migration friction for existing flows
  that relied on broad implicit access.
- Approval latency can become an operational bottleneck if override paths are
  too coarse.
- Bundle-level defaults can accidentally hide privilege escalation unless the
  review surface remains explicit.

## Outstanding Decisions

- Who can approve privilege overrides
- Whether development-mode overrides are persisted or remain ephemeral
- How security policy interacts with bundle-defined defaults
- Whether some capability combinations remain permanently unsupported

## Architectural Rationale

The companion design[^1] increases the number of host-adjacent capabilities
that Corbusier and Podbot can expose. Least-privilege defaults keep that
surface coherent, and explicit overrides preserve the review and audit
authority that Corbusier is meant to retain.

[^1]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
