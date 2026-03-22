# Architectural decision record (ADR) 001: runtime boundary between Corbusier and Podbot

## Status

Proposed.

## Date

2026-03-21.

## Context and Problem Statement

Corbusier is moving from a conceptual integration with Podbot towards direct
conformance with Podbot's library API surface, as described in
`docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The current
repository still reflects an older model in which Corbusier can act as both the
policy layer and part of the runtime host for agent sessions.

That ambiguity makes later design work unstable. Workspace lifecycle, Model
Context Protocol (MCP) wires, hook execution, prompt validation, and audit
capture all depend on a single answer to a foundational question: which system
owns runtime mechanics for hosted sessions?

This ADR sets that boundary. It does not define the workspace model, the wire
model, or the hook protocol in detail. Those topics are owned by later ADRs and
depend on this one.

## Decision Drivers

- Single ownership of runtime mechanics
- Clear separation between policy and execution
- Type safety and testability at the integration seam
- Predictable control-channel behaviour
- Reduced duplication between Corbusier and Podbot
- Operator clarity during failure diagnosis and recovery

## Requirements

### Functional requirements

- Corbusier must be able to ask Podbot to prepare workspaces.
- Corbusier must be able to ask Podbot to launch and stop hosted agents.
- Corbusier must be able to request MCP wires and receive hook messages.
- Corbusier must be able to validate prompts against agent targets through the
  Podbot surface.

### Technical requirements

- The accepted design must not depend on parsing command-line interface (CLI)
  text output as the primary integration method.
- The accepted design must define distinct control channels for runtime events
  that cannot share the agent protocol stream.
- The accepted design must support deterministic fake adapters for tests.
- The accepted design must define typed error handling between Corbusier and
  Podbot.

## Goals and Non-Goals

### Goals

- Ratify Podbot as the normative hosted runtime surface.
- Keep Corbusier as the policy, catalogue, orchestration, and audit authority.
- Make the library API the primary seam for runtime operations.

### Non-Goals

- Recreate Podbot internals inside Corbusier.
- Specify the detailed workspace lifecycle model.
- Specify the hook schema, wire schema, or persistence schema.

## Podbot roadmap dependencies

This ADR depends on the following upstream Podbot roadmap steps:

- Step 1.4, "Hosting schema migration and compatibility matrix", because the
  library-facing hosting configuration must exist before Corbusier can treat
  the library API as the normative runtime surface.
- Step 4.3b, "App server startup", because the hosted app-server path is part
  of the runtime boundary that this ADR assigns to Podbot.
- Step 4.5, "Normalized launch contract", because Corbusier needs one typed
  launch seam rather than a mix of ad hoc runtime entry points.
- Step 4.6, "Hosted session control plane", because Corbusier depends on a
  typed control and event surface instead of CLI scraping.

## Options Considered

### Option A: Podbot is the sole runtime owner

Podbot owns workspace shaping, container lifecycle, MCP wire bridging, and
generic hook execution. Corbusier calls Podbot through a typed library-facing
adapter and retains policy, registry, orchestration, and durable state.

### Option B: Corbusier wraps the Podbot CLI

Corbusier treats the Podbot CLI as the stable interface and parses command
output to discover state transitions, hook requests, and other runtime events.

### Option C: Corbusier and Podbot split runtime duties

Corbusier keeps selected execution responsibilities, such as direct hook
execution or some workspace mechanics, while Podbot handles only the remaining
runtime tasks.

| Topic                     | Option A                | Option B                 | Option C            |
| ------------------------- | ----------------------- | ------------------------ | ------------------- |
| Runtime ownership         | Clear single owner      | Implicit and brittle     | Split and ambiguous |
| Integration seam          | Typed library API       | Parsed process output    | Mixed seams         |
| Testability               | High with fake adapters | Low due to text coupling | Medium              |
| Failure diagnosis         | Clear boundaries        | Noisy and indirect       | Diffuse             |
| Long-term maintainability | Strong                  | Weak                     | Weak                |

_Table 1: Trade-offs for the runtime ownership boundary._

## Decision Outcome / Proposed Direction

Corbusier should treat Podbot as the sole runtime owner for hosted sessions and
should use the Podbot library API as the normative integration surface.

Under this direction:

- Podbot owns workspace shaping, container lifecycle, MCP wire bridging, and
  generic hook execution.
- Corbusier owns policy, registry, orchestration, durable state, prompt and
  bundle selection, and audit interpretation.
- Runtime events that cannot travel on the agent protocol stream must use
  explicit control channels between Podbot and Corbusier.
- Test adapters must mirror the Podbot boundary, not recreate a second
  production-grade runtime inside Corbusier.

## Migration Plan

### Phase 1

Define a Corbusier port for the Podbot library API and route new hosted-agent
flows through that port.

### Phase 2

Move workspace preparation, wire provisioning, hook execution, and prompt
validation responsibilities behind the Podbot-facing adapter.

### Phase 3

Remove Corbusier-owned runtime code paths for Podbot-hosted sessions once the
replacement path is stable and covered by tests.

## Known Risks and Limitations

- A CLI fallback may still be useful for development or recovery tooling, but
  it must remain explicitly non-normative.
- Retry ownership between Corbusier and Podbot must be defined carefully to
  avoid duplicate side effects.
- Test doubles can accidentally become a shadow runtime if they model too much
  production behaviour.

## Outstanding Decisions

- Whether a CLI fallback is permitted for development and recovery tooling
- Which side owns low-level retry policy for runtime actions
- Whether any runtime action remains valid without Podbot in the loop

## Architectural Rationale

This boundary preserves a clean hexagonal split. Corbusier remains the system
of record for intent, policy, and audit, while Podbot becomes the execution
engine for hosted sessions. That improves conformance with the companion design
and prevents later ADRs from re-opening the question of runtime ownership.
