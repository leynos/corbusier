# Repository layout

This document describes the major paths in the Corbusier repository and the
responsibilities attached to each. The tree is intentionally compact; it
highlights paths that contributors normally need for orientation rather than
every generated or tool-owned file.

```plaintext
.
├── .agents/
├── .cargo/
├── .config/
├── .factory/
├── .github/
├── charts/
├── docs/
│   ├── execplans/
│   └── rfcs/
├── frontend-pwa/
├── migrations/
├── scripts/
├── src/
├── tests/
├── AGENTS.md
├── Cargo.toml
├── Makefile
└── README.md
```

_Figure 1: Simplified Corbusier repository layout._

## Path responsibilities

| Path                          | Responsibility                                                                                            |
| ----------------------------- | --------------------------------------------------------------------------------------------------------- |
| `.agents/`                    | Agent-facing configuration and Model Context Protocol (MCP) material used by local development workflows. |
| `.cargo/`                     | Cargo configuration for workspace builds and linting behaviour.                                           |
| `.config/`                    | Tool configuration, including the nextest profile used by `make test`.                                    |
| `.factory/`                   | Factory Droid plugin configuration for local development support.                                         |
| `.github/`                    | GitHub automation such as Dependabot and workflow definitions.                                            |
| `charts/`                     | Kubernetes and Helm packaging material for Corbusier deployment experiments.                              |
| `docs/`                       | Long-lived documentation, design records, guides, roadmaps, and plans.                                    |
| `docs/execplans/`             | Living execution plans for non-trivial implementation work.                                               |
| `docs/rfcs/`                  | Request for Comments documents for proposed technical changes.                                            |
| `frontend-pwa/`               | Progressive web application source, tests, tooling, and generated distribution output.                    |
| `migrations/`                 | Diesel database migrations for persistent Corbusier state.                                                |
| `scripts/`                    | Repository scripts and support SQL used by maintainer workflows.                                          |
| `src/`                        | Rust library, binary, adapters, domain modules, and worker entry points.                                  |
| `tests/`                      | Integration, behavioural, fixture, helper, in-memory, and PostgreSQL test support.                        |
| `AGENTS.md`                   | Normative agent and contributor instructions for this repository.                                         |
| `Cargo.toml` and `Cargo.lock` | Rust workspace manifest and locked dependency graph.                                                      |
| `Makefile`                    | Canonical local quality gate and development command surface.                                             |
| `README.md`                   | Public project overview, quick start, and contributor entrypoint.                                         |

_Table 1: Repository paths and their responsibilities._

## Source and test layout

Rust source code lives under `src/`. Feature-oriented modules such as
`agent_backend`, `context`, `health`, `hook_engine`, `http_api`, `message`,
`task`, `tenant`, and `tool_registry` keep domain logic close to the adapters
and helpers that serve that feature. Binary and worker entry points live in
`src/main.rs`, `src/bin/`, and `src/worker.rs`.

Integration and behavioural coverage lives under `tests/`. Scenario entry
points sit beside step modules, fixtures, in-memory adapters, PostgreSQL
support, and shared test helpers so that test ownership remains visible at the
feature boundary.

## Documentation layout

Long-lived documentation belongs in `docs/`. Use
[documentation contents](contents.md) to choose the correct destination before
adding or updating a document. Design intent belongs in design documents,
accepted decisions belong in ADRs, proposed changes belong in `docs/rfcs/`, and
implementation plans belong in `docs/execplans/`.

## Generated and tool-owned paths

Do not treat generated artefacts as source of truth. The `frontend-pwa/dist/`
directory is build output. Cargo build output belongs in `target/`, which is
not part of the source tree. Tool caches, local service state, and temporary
logs should remain outside the repository unless a documented workflow
explicitly requires checked-in fixtures.
