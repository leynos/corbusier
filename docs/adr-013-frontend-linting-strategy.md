# Architectural decision record (ADR) 013: Frontend linting strategy

## Status

Accepted on 26 October 2026. The frontend lint command runs Biome followed by
Oxlint, with the recommended Unicorn rules and the `ai-deslop` plugin enabled.

## Date

2026-10-26.

## Context and problem statement

The repository-owned frontend already uses Biome and project-specific checks.
It needs additional JavaScript and TypeScript rules for code quality and safer
handling of data received from APIs, configuration files, and tools. The added
checks must preserve existing Biome requirements, remain reproducible, and
surface type-aware findings in the same frontend lint command.

## Decision drivers

- Apply the recommended Unicorn profile to the frontend workspace.
- Add the requested `maharshi365/deslop` rules without weakening existing
  Biome or TypeDoc checks.
- Pin the lint tools and external plugin so rule changes are reviewed with
  version updates.
- Keep exceptions narrow and record direct rule conflicts explicitly.
- Validate untrusted input at its boundary rather than relying on unchecked
  shape assumptions.

## Options considered

- Keep Biome alone. This does not provide the requested Unicorn and deslop
  rules.
- Add Oxlint with floating versions. This would allow rule and plugin changes
  without a reviewed dependency update.
- Add Oxlint with exact versions and enumerate the adopted rules. This keeps
  the selected profile reviewable and makes exceptions visible in the config.

## Decision outcome

`frontend-pwa/package.json` pins Oxlint at `1.82.0`, `ai-deslop` at `0.4.9`, and
`oxlint-tsgolint` at `7.0.2004`. `ai-deslop` is loaded as an Oxlint JavaScript
plugin. Oxlint runs with type-aware rules enabled. The frontend `lint` script
runs `biome check .` first and `oxlint .` second.

`frontend-pwa/.oxlintrc.json` explicitly enables the recommended Unicorn rule
set and the selected deslop rules at error severity. Explicit rule entries make
the adopted profile and its review surface visible next to the tool
configuration. The single disabled rule is `deslop/no-reflect-apply`, because
it conflicts with `unicorn/prefer-reflect-apply`; Unicorn's recommendation
takes precedence. No other rules are disabled.

Oxlint's type-aware checks use the pinned `oxlint-tsgolint` package. Runtime
schemas belong at the boundary where the frontend or its tooling reads external
data. The task HTTP adapter, audit parser, and frontend configuration scripts
use Zod schemas so validated values have an explicit contract before the
application inspects them. The task schema retains unknown fields so additive
API changes remain available to callers.

Biome remains the first frontend lint stage and retains its existing rules and
plugins. TypeDoc remains a separate zero-tolerance documentation gate; neither
tool is weakened to accommodate Oxlint.

## Consequences

- Contributors run `make frontend-lint` to execute Biome and Oxlint in order.
- Changes to either exact Oxlint package version require reviewing the
  configured rule set and its exceptions.
- Type-aware linting relies on the pinned `oxlint-tsgolint` executable and the
  TypeScript project configuration supplied by the frontend workspace.
- A future Unicorn or deslop rule conflict must be handled with one narrow
  documented exception, preferring Unicorn when the conflict is between those
  two plugins.
