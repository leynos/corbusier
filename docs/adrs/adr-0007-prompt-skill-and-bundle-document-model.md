# Architectural decision record (ADR) 007: prompt, skill, and bundle document model

## Status

Proposed.

## Date

2026-03-21

## Context and Problem Statement

The Podbot conformance work introduces a new repository-facing document
surface. Prompts, skill directories, and higher-level skill bundles are no
longer incidental configuration. They become part of the stable integration
contract for hosted agents, workspace-scoped wires, and hook-aware execution.

Corbusier therefore needs a document model that preserves compatibility with
the wider Agent Skills ecosystem while adding a first-class bundle abstraction
and structured prompt files with predictable rendering semantics.

This ADR owns the prompt file shape, skill bundle abstraction, normative
frontmatter vocabulary, and template-rendering scope. Capability validation is
owned separately by ADR 008.

## Decision Drivers

- Portability of skills
- Progressive disclosure and context discipline
- Predictable prompt rendering
- Clear policy integration for hooks and MCP wires
- Compatibility with different hosted-agent targets

## Requirements

### Functional requirements

- The design must define a bundle abstraction that groups skills, prompts, and
  related runtime dependencies.
- The design must define a prompt document shape with harmonized frontmatter.
- The design must define which frontmatter fields are normative.
- The design must define how skills are attached, selected, or preloaded.

### Technical requirements

- The design must define which fields are eligible for template interpolation.
- The design must define template rendering rules, error behaviour, and
  undefined-variable semantics.
- The design must define extension namespaces for repository-specific metadata.
- The design must define whether bundles are repository-local, registry-backed,
  or both.

## Goals and Non-Goals

### Goals

- Keep standard skill directories compatible with the wider ecosystem.
- Add a Corbusier bundle abstraction above skills and prompts.
- Give prompt files a structured, reviewable, and machine-readable format.

### Non-Goals

- Invent a Corbusier-only replacement for portable skill directories.
- Collapse prompt content, bundle metadata, and skills into one file type.
- Finalize validation outcomes for target runtimes.

## Podbot roadmap dependencies

This ADR depends on the following upstream Podbot roadmap steps:

- Step 4.5, "Normalized launch contract" because prompts, bundles, skill
  selection, hook subscriptions, and wire references must feed one normalized
  launch plan.
- Step 4.8, "Prompt, bundle, and validation surfaces" because Podbot's prompt
  and bundle contracts need to be documented in a form that can align with the
  Corbusier document model defined here.

## Options Considered

### Option A: Preserve standard skill directories and add first-class bundles and prompt files

Corbusier keeps skill directories in their standard portable form, introduces a
bundle abstraction that groups them with prompts and runtime dependencies, and
defines prompt files as structured Markdown with harmonized frontmatter.

### Option B: Define a Corbusier-only prompt and bundle format

Corbusier replaces the wider skill ecosystem with a repository-specific format
for both bundles and prompts.

### Option C: Collapse bundles and prompts into one Markdown artefact

Corbusier stores prompt content, skill declarations, and bundle metadata in the
same document type and relies on conventions to distinguish roles.

Table 1: Trade-offs for the prompt and bundle document model.

| Topic                     | Option A | Option B | Option C |
| ------------------------- | -------- | -------- | -------- |
| Skill portability         | Strong   | Weak     | Medium   |
| Review clarity            | Strong   | Medium   | Weak     |
| Extension flexibility     | Strong   | Medium   | Weak     |
| Ecosystem fit             | Strong   | Weak     | Weak     |
| Implementation simplicity | Medium   | Medium   | High     |

## Decision Outcome / Proposed Direction

Corbusier should preserve standard skill directories, add a first-class bundle
abstraction above them, and define prompt files as structured Markdown with
harmonized frontmatter and Goose-compatible Jinja2 interpolation semantics.

The proposed document model is:

- skill directories remain portable and compatible with the wider ecosystem,
- bundles group prompts, skill selections, wire defaults, hook defaults, and
  related runtime dependencies,
- prompt files carry normative frontmatter for runtime requirements and
  attachment references, and
- repository-specific additions use namespaced extension fields instead of
  ad hoc free-form metadata.

Frontmatter should remain mostly literal, with interpolation limited to fields
that are explicitly marked as templated. Undefined variables should be treated
as validation errors unless a field is declared optional by the document model.

## Migration Plan

This ADR lands during ADR 010 Phase 2 (durability and document surfaces). The
implementation steps below are scoped to this ADR; see ADR 010 for the
cross-cutting migration sequence and advancement criteria.

### Phase 1

Define the prompt document shape, bundle manifest shape, and normative
frontmatter fields.

### Phase 2

Add repository parsing, rendering, and attachment logic for prompts, skills,
and bundles.

### Phase 3

Move tests, examples, and public repository-facing documentation onto the new
bundle and prompt surfaces.

## Known Risks and Limitations

- Over-templating frontmatter can make review difficult and validation
  unpredictable.
- Bundle-level defaults can obscure prompt-level intent if override rules are
  not clear.
- Registry-backed bundles introduce immutability and versioning questions that
  the initial repository-local model may not solve completely.

## Outstanding Decisions

- Whether frontmatter itself is templated or strictly literal by default
- Whether bundles may carry runtime defaults such as hooks and wires
- Whether prompt files may override bundle-level defaults
- How versioning and immutability are represented for reviewed artefacts

## Architectural Rationale

This direction keeps Corbusier aligned with the external skill ecosystem while
adding the higher-level grouping that the companion design[^1] requires. It
also gives later validation and security work a stable, typed document surface
rather than a loose collection of Markdown conventions.

[^1]: The companion design is
    `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`. The
    Podbot roadmap steps referenced in this ADR are defined in the upstream
    [Podbot roadmap](https://github.com/leynos/podbot/blob/main/docs/podbot-roadmap.md).
