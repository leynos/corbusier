# Architectural decision record (ADR) 004: canonical MCP source taxonomy and legacy transport migration

## Status

Proposed.

## Date

2026-03-21.

## Context and Problem Statement

ADR 003 proposes that Podbot-hosted agents consume workspace-scoped wires
directly. That model depends on a clean distinction between three separate
concepts:

- a persisted source definition that Corbusier owns,
- a workspace-scoped wire instance that Podbot realises, and
- an agent-visible endpoint that the hosted runtime consumes.

Corbusier's current transport vocabulary is too close to older integration
detail and does not cleanly separate those concepts. The repository therefore
needs one canonical source taxonomy and one migration story for legacy
transport records.

## Decision Drivers

- Clear persistence semantics for source definitions
- Compatibility with workspace wire provisioning
- Explicit trust boundary for helper containers
- Controlled migration from legacy transport shapes

## Requirements

### Functional requirements

- The source taxonomy must distinguish local stdio, helper-container stdio,
  and direct Streamable Hypertext Transfer Protocol (HTTP) sources.
- The taxonomy must include explicit helper-container repository access
  controls.
- The taxonomy must support source-specific health, readiness, and validation
  behaviour.

### Technical requirements

- The design must define how legacy records are parsed and re-serialized.
- The design must prevent the source definition from being confused with a
  workspace wire endpoint.
- The design must define whether any legacy transport names remain accepted in
  storage or only in migration code.

## Goals and Non-Goals

### Goals

- Replace ambiguous transport labels with a stable source taxonomy.
- Keep source definitions independent of workspace wire instances.
- Make helper-container trust boundaries explicit in the model.

### Non-Goals

- Define the runtime wire lifecycle itself.
- Preserve every legacy label as a first-class design term.
- Decide health aggregation for whole workspaces or sessions.

## Options Considered

### Option A: Replace the legacy transport model with a new source taxonomy

Corbusier adopts a canonical source taxonomy and treats legacy transport labels
as compatibility-only input shapes during migration.

### Option B: Extend the legacy transport model

Corbusier keeps the existing transport vocabulary and adds more variants to it
for helper containers and new Podbot-aligned sources.

### Option C: Keep the existing model and rely on documentation

Corbusier preserves the current transport model and asks readers and
maintainers to infer the difference between source definitions and wire
instances from surrounding prose.

| Topic                     | Option A | Option B | Option C |
| ------------------------- | -------- | -------- | -------- |
| Type clarity              | Strong   | Medium   | Weak     |
| Migration complexity      | Medium   | Medium   | Low      |
| Long-term maintainability | Strong   | Medium   | Weak     |
| Wire-model alignment      | Strong   | Medium   | Weak     |
| Legacy ambiguity          | Low      | Medium   | High     |

_Table 1: Trade-offs for the canonical MCP source model._

## Decision Outcome / Proposed Direction

Corbusier should replace the legacy transport model with a canonical source
taxonomy and treat legacy transport labels as compatibility-only constructs.

The canonical taxonomy should distinguish:

- local stdio sources,
- helper-container stdio sources, and
- direct Streamable HTTP sources.

The model must also record helper-container repository access explicitly, such
as no repository access, read-only access, or read-write access. Agent-visible
wire endpoints must not be persisted as source definitions because they are
workspace-specific runtime artefacts.

Legacy transport labels may be accepted during migration input and converted to
the canonical model, but they should not remain normative storage terms once
the migration completes.

## Migration Plan

### Phase 1

Define the canonical source taxonomy in Corbusier's domain model and adapters.

### Phase 2

Add compatibility parsing for legacy transport records and write canonical
records back out on update paths.

### Phase 3

Stop writing legacy transport labels and confine them to migration and import
code paths before eventual removal.

## Known Risks and Limitations

- Some legacy records may not map cleanly without operator review.
- Health and readiness semantics can drift if source-level and wire-level
  checks are not kept distinct.
- Helper-container repository access defaults can become an accidental
  privilege-escalation path if left implicit.

## Outstanding Decisions

- How much automatic migration is safe for ambiguous legacy records
- Whether partial legacy reads remain supported during transition
- Which health semantics belong to the source and which belong to the wire
- Whether helper-container sources may ever inherit repository access by
  default

## Architectural Rationale

The companion design assumes that source definitions, wires, and agent-visible
endpoints are distinct concepts. A canonical source taxonomy makes that
distinction explicit in Corbusier's types, persistence model, and review
language, which reduces confusion in both implementation and future ADRs.
