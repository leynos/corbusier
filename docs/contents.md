# Documentation contents

This index lists the long-lived project documentation for Corbusier. Use it as
the first stop when choosing which document to read or update.

## Index

- [Documentation contents](contents.md) identifies the documentation set and
  keeps navigation stable as files are added, renamed, or removed.
- [Repository layout](repository-layout.md) explains the purpose and ownership
  boundaries of the major paths in the repository.
- [Documentation style guide](documentation-style-guide.md) defines the
  spelling, Markdown, diagram, roadmap, RFC, and ADR conventions used by this
  project.
- [Users' guide](users-guide.md) explains user-facing Corbusier behaviour and
  runtime expectations.
- [Developers' guide](developers-guide.md) explains maintainer workflows,
  quality gates, local services, and implementation conventions.
- [Roadmap](roadmap.md) tracks planned delivery work, dependencies, and
  acceptance criteria.

## Architecture and design

- [Corbusier design](corbusier-design.md) is the primary system design document
  for the orchestration platform.
- [Corbusier API design](corbusier-api-design.md) describes the HTTP API,
  contracts, and integration surface.
- [Local Kubernetes preview design](local-k8s-preview-design.md) describes the
  local preview environment for development and validation.
- [Podbot conformance design for agents, MCP wires, and hooks](podbot-conformance-design-for-agents-mcp-wires-and-hooks.md)
  records the interoperability contract with Podbot.
- [Podbot migration review checklist](podbot-migration-review-checklist.md)
  supports review of migration work against the Podbot contract.

## Decision records

- [ADR 001: Runtime boundary between Corbusier and Podbot](adr-001-runtime-boundary-between-corbusier-and-podbot.md)
  records ownership of runtime responsibilities.
- [ADR 002: Workspace runtime model and source policy](adr-002-workspace-runtime-model-and-source-policy.md)
  records workspace source and execution policy.
- [ADR 003: MCP wire model and tool plane ownership](adr-003-mcp-wire-model-and-tool-plane-ownership.md)
  records MCP transport and tool-plane boundaries.
- [ADR 004: Canonical MCP source taxonomy and legacy transport migration](adr-004-canonical-mcp-source-taxonomy-and-legacy-transport-migration.md)
  records source taxonomy and migration handling.
- [ADR 005: Hook execution contract and control channel semantics](adr-005-hook-execution-contract-and-control-channel-semantics.md)
  records hook control-channel behaviour.
- [ADR 006: Durable runtime state and audit model](adr-006-durable-runtime-state-and-audit-model.md)
  records persistence and audit expectations.
- [ADR 007: Prompt, skill, and bundle document model](adr-007-prompt-skill-and-bundle-document-model.md)
  records document model ownership.
- [ADR 008: Prompt validation semantics and capability dispositions](adr-008-prompt-validation-semantics-and-capability-dispositions.md)
  records validation and capability-disposition behaviour.
- [ADR 009: Security and privilege boundary defaults](adr-009-security-and-privilege-boundary-defaults.md)
  records default security boundaries.
- [ADR 010: Migration and coexistence strategy](adr-010-migration-and-coexistence-strategy.md)
  records migration and coexistence policy.
- [ADR 011: Corbusier and Frankie review adapter role segregation](adr-011-corbusier-frankie-review-adapter-role-segregation.md)
  records adapter responsibility boundaries.

## Guides and reference material

- [Complexity antipatterns and refactoring strategies](complexity-antipatterns-and-refactoring-strategies.md)
  describes maintainability risks and refactoring responses.
- [Dependency policy exception: Actix v2a](dependency-policy-exception-actix-v2a.md)
  records the documented dependency-policy exception.
- [Ortho config users' guide](ortho-config-users-guide.md) documents Ortho
  configuration usage.
- [pg-embed setup for unprivileged users](pg-embed-setup-unpriv-users-guide.md)
  explains local PostgreSQL setup constraints.
- [Reliable testing in Rust via dependency injection](reliable-testing-in-rust-via-dependency-injection.md)
  describes testability patterns for Rust code.
- [Roadmap writing guide](roadmap-writing-guide.md) documents roadmap structure
  and task-writing conventions.
- [rstest-bdd users' guide](rstest-bdd-users-guide.md) documents behavioural
  testing conventions.
- [Rust doctest DRY guide](rust-doctest-dry-guide.md) explains doctest reuse
  patterns.
- [Rust testing with rstest fixtures](rust-testing-with-rstest-fixtures.md)
  documents fixture-based testing practices.
- [Scripting standards](scripting-standards.md) defines conventions for project
  scripts.
- [Whitaker users' guide](whitaker-users-guide.md) explains local Whitaker
  linting setup and usage.

## Planning directories

- [Execution plans](execplans/) contains living implementation plans for
  non-trivial changes.
- [Requests for Comments](rfcs/) contains proposed technical changes that need
  review before acceptance.
