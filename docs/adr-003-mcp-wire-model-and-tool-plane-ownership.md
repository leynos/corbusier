# Architectural decision record (ADR) 003: MCP wire model and tool-plane ownership

## Status

Proposed.

## Date

2026-03-21

## Context and Problem Statement

Corbusier already contains a Model Context Protocol (MCP) server registry and a
tool routing model. The Podbot conformance design shifts the runtime shape:
agents should consume workspace-scoped MCP wires that Podbot provisions, while
Corbusier remains the catalogue and policy authority.

That creates a decision boundary that must be settled exactly once. For
Podbot-hosted agents, does Corbusier stay in the tool call path, or does the
agent talk to workspace-scoped MCP wires directly?

This ADR owns the tool-plane boundary between Corbusier's catalogue and
Podbot's workspace wire provisioning. It does not define the source taxonomy
for MCP definitions; ADR 004 owns that lower-level model.

## Decision Drivers

- Conformance with the planned Podbot interface
- Reduction of duplicate routing layers
- Clear tenancy and workspace scoping
- Predictable tool availability at session start
- Auditability without proxying every call through Corbusier

## Requirements

### Functional requirements

- Corbusier must catalogue available MCP sources independently of any one
  workspace.
- Corbusier must be able to provision a named wire set per workspace.
- Hosted agents must receive a consistent view of the wires attached to their
  workspace.
- Policy must determine which wires exist for a given task or prompt.

### Technical requirements

- Wire creation and teardown must be idempotent.
- Wire identity must be distinct from source identity.
- The accepted model must define how the agent learns endpoint Uniform Resource
  Locators (URLs) and associated headers.
- The accepted model must define where tool-call logs originate and how they
  reach Corbusier.

## Goals and Non-Goals

### Goals

- Keep Corbusier as the policy and catalogue authority.
- Let Podbot own the runtime wire plane for hosted sessions.
- Avoid mixed routing models for the same hosted-agent session.

### Non-Goals

- Redefine MCP source shapes or persistence schemas.
- Force all non-Podbot integrations to use the same runtime path.
- Specify the detailed telemetry schema for tool calls.

## Podbot roadmap dependencies

This ADR depends on the following upstream Podbot roadmap steps:

- Step 4.5, "Normalized launch contract", because wire selection and injection
  details need to be normalized before launch.
- Step 4.6, "Hosted session control plane", because Corbusier needs typed
  runtime events for wire status without moving tool calls back into its inline
  path.
- Step 4.7, "MCP wire provisioning and injection", because that step is the
  direct upstream Podbot surface that materializes workspace-scoped wires.

## Options Considered

### Option A: The hosted agent is the MCP client

Podbot provisions workspace-scoped MCP wires and injects the resulting
endpoints into the hosted runtime. Corbusier decides which sources may be
attached and records the resulting wire set.

### Option B: Corbusier remains the inline tool caller

The hosted agent sends tool requests back through Corbusier, which stays in the
runtime call path and dispatches work to MCP servers indirectly.

### Option C: Mixed mode

Some tools are exposed as direct MCP wires while others remain proxied through
Corbusier for the same hosted-agent session.

Table 1: Trade-offs for tool-plane ownership in Podbot-hosted sessions.

| Topic                      | Option A | Option B | Option C |
| -------------------------- | -------- | -------- | -------- |
| Podbot conformance         | Strong   | Weak     | Weak     |
| Runtime simplicity         | Strong   | Medium   | Weak     |
| Audit ingestion complexity | Medium   | Low      | High     |
| Duplicate routing layers   | None     | High     | High     |
| Session predictability     | Strong   | Medium   | Weak     |

## Decision Outcome / Proposed Direction

For Podbot-hosted agents, the hosted agent should be the MCP client. Podbot
should provision workspace-scoped wires, and Corbusier should remain the tool
catalogue and policy authority rather than the inline runtime caller.

Under this model:

- Corbusier stores source definitions and policy rules.
- Corbusier asks Podbot to materialize a wire set for a specific workspace.
- Podbot injects agent-visible endpoints and required headers into the hosted
  runtime.
- Corbusier receives tool-call telemetry and lifecycle events out of band for
  audit and operator visibility.
- Mixed-mode invocation is out of scope for the initial conformant path and
  should be treated as a separate exception model if it is ever justified.

## Migration Plan

This ADR lands during ADR 010 Phase 1 (foundational architecture). The
implementation steps below are scoped to this ADR; see ADR 010 for the
cross-cutting migration sequence and advancement criteria.

### Phase 1

Separate Corbusier's catalogue concepts from runtime wire attachment concepts.

### Phase 2

Add Podbot-facing wire provisioning for hosted sessions and publish the agent's
wire view at session start.

### Phase 3

Retire direct inline tool-routing assumptions for Podbot-hosted sessions once
telemetry and audit ingestion are in place.

## Known Risks and Limitations

- Audit collection becomes an event-ingestion problem instead of a by-product
  of inline routing.
- Dynamic wire attachment during a live session would complicate agent
  expectations and lifecycle handling.
- Discovery metadata can drift if source changes are not reconciled against
  active ephemeral wire state.

## Outstanding Decisions

- How tool-call telemetry enters Corbusier when Corbusier is not inline
- Whether mixed-mode tool invocation is ever justified
- Whether wire attachment may change during a live hosted-agent session
- How discovery metadata stays consistent with ephemeral wire state

## Architectural Rationale

This direction preserves the control-plane and data-plane split proposed in the
companion design[^cd]. Corbusier decides what is allowed. Podbot makes those
decisions concrete inside a specific workspace runtime. That keeps the runtime
path narrow and avoids building a second routing layer on top of Podbot.

[^cd]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
