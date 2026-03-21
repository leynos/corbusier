# Architectural decision record (ADR) 008: prompt validation semantics and capability dispositions

## Status

Proposed.

## Date

2026-03-21.

## Context and Problem Statement

ADR 007 defines the prompt, skill, and bundle document surface. Corbusier now
needs a validation model that tells the truth about how a specific hosted-agent
target will interpret that surface.

The Podbot conformance design requires more than a binary valid-versus-invalid
answer. Some prompt capabilities may be native to the target agent, some may be
host-enforced, some may be translated, and some may be ignored or rejected.
Continuous Integration (CI), operator review, and runtime safety all depend on
that distinction being visible before execution starts.

This ADR owns the validation request and response shape, capability vocabulary,
and disposition reporting model.

## Decision Drivers

- Truthful operator feedback
- Safer CI gating
- Predictable degradation semantics across agent types
- Reviewability of prompt and bundle changes
- Stable vocabulary for validation results

## Requirements

### Functional requirements

- The design must define the validation request shape.
- The design must define the validation response shape.
- The design must define disposition categories for capabilities.
- The design must define which dispositions block execution and which allow
  degraded execution.
- The design must define whether the validator returns an effective prompt
  preview.

### Technical requirements

- Validation must be deterministic for a given agent target and prompt input.
- The design must separate warnings from errors.
- The design must define how prompt input schema errors are surfaced.
- The design must define how validation interacts with unresolved template
  variables.

## Goals and Non-Goals

### Goals

- Give operators a structured answer instead of free-text guesswork.
- Report degraded execution explicitly rather than hiding it behind warnings.
- Keep validation at the Podbot integration seam where target behaviour is
  known.

### Non-Goals

- Guarantee that runtime provisioning errors are caught by validation alone.
- Replace orchestration-time checks for missing runtime resources.
- Encode every implementation detail of every hosted-agent backend.

## Options Considered

### Option A: Capability-disposition validation with structured diagnostics

Validation returns explicit capability dispositions, typed diagnostics, and an
optional effective prompt preview after rendering and capability filtering.

### Option B: Boolean validation with free-text warnings

Validation returns only pass or fail plus human-readable warning text.

### Option C: Validation deferred to runtime

No preflight validation exists beyond parsing. The hosted runtime exposes
problems only when the session starts or fails.

| Topic                   | Option A | Option B | Option C |
| ----------------------- | -------- | -------- | -------- |
| Operator clarity        | Strong   | Medium   | Weak     |
| CI suitability          | Strong   | Weak     | Weak     |
| Capability truthfulness | Strong   | Weak     | Weak     |
| Implementation effort   | Medium   | Low      | Low      |
| Reviewability           | Strong   | Weak     | Weak     |

_Table 1: Trade-offs for prompt validation semantics._

## Decision Outcome / Proposed Direction

Corbusier should adopt structured validation with explicit capability
dispositions and typed diagnostics.

The initial disposition vocabulary should include:

- native,
- host-enforced,
- translated,
- ignored, and
- rejected.

Validation results should also state whether a disposition blocks execution or
permits degraded execution. Required capabilities that end in `ignored` or
`rejected` should block execution. Preferred capabilities may degrade with a
warning. Forbidden capabilities should fail validation if the target or host
would enable them.

The validator should return an effective prompt preview when rendering and
capability filtering complete successfully. Unresolved template variables or
prompt schema errors should be reported as structured errors, not buried in
free-text warnings.

## Migration Plan

### Phase 1

Define the capability vocabulary, request shape, response shape, and diagnostic
categories.

### Phase 2

Implement validation through the Podbot-facing adapter for one hosted-agent
target and persist fixtures that exercise degraded cases.

### Phase 3

Adopt validation results in repository review tooling and CI gates, moving from
warn-only to blocking behaviour where the migration plan allows.

## Known Risks and Limitations

- Capability vocabularies can drift if prompt authors and runtime implementers
  do not use the same terms.
- Effective prompt previews may be mistaken for runtime guarantees if resource
  checks remain orchestration-time concerns.
- Some backend-specific degradation paths may still require human review even
  with structured diagnostics.

## Outstanding Decisions

- Whether validation is exposed only through the library surface or also
  through a user-facing command
- Whether prompt rendering happens before validation, after validation, or in a
  staged mixed flow
- Whether missing wires or hook subscriptions are validation errors or
  orchestration errors
- Whether validation results are persisted for audit

## Architectural Rationale

The companion design treats validation as a product surface rather than an
internal convenience. Structured dispositions match that requirement, support
CI and design review, and make capability degradation explicit instead of
turning it into hidden runtime surprise.
