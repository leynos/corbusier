# Podbot migration review checklist

This maintainer-facing checklist is the operational companion for roadmap items
`1.1.1` and `1.6.1`. Use it when reviewing pull requests that claim
compatibility, warn-only, or blocking phase advancement for the Podbot-hosted
migration.

## 1. Source-of-truth documents

Review phase advancement against these documents together:

- `docs/adr-001-runtime-boundary-between-corbusier-and-podbot.md`
- `docs/adr-002-workspace-runtime-model-and-source-policy.md`
- `docs/adr-003-mcp-wire-model-and-tool-plane-ownership.md`
- `docs/adr-004-canonical-mcp-source-taxonomy-and-legacy-transport-migration.md`
- `docs/adr-005-hook-execution-contract-and-control-channel-semantics.md`
- `docs/adr-010-migration-and-coexistence-strategy.md`
- `docs/corbusier-design.md`
- `docs/roadmap.md`

Reject the pull request if those documents disagree about ownership,
dependencies, or the current migration stage.

## 2. Boundary invariants

Reject the pull request if it revives any of these retired ownership patterns
for Podbot-hosted sessions:

- Corbusier directly owns workspace shaping, container lifecycle, or hosted
  launch mechanics.
- Corbusier stays in the inline tool-call path instead of acting as the policy
  and catalogue authority for Podbot-provisioned wires.
- Corbusier executes hosted hooks locally instead of governing Podbot-owned
  hook execution through acknowledgements and audit.

The accepted boundary is:

- Podbot owns hosted runtime mechanics, workspace realization, wire bridging,
  generic hook execution, and the hosted-session control surface.
- Corbusier owns policy, registry, orchestration, durable state, prompt and
  bundle selection, and audit interpretation.

## 3. Compatibility stage

Approve compatibility-stage advancement only when all of the following are true:

- ADRs 001 through 005 and ADR 010 are accepted and unchanged in boundary
  substance.
- The pull request routes the new surface through a Podbot-facing seam or a
  clearly named compatibility adapter.
- New work does not depend on inline hosted runtime ownership in Corbusier.
- Canonical write paths exist for the changed surface, or the pull request
  explicitly documents why it is still read-only groundwork.
- Compatibility reads for retained historical data are either already proven
  or tracked as a dependency owned by a later roadmap item.

Do not advance the stage if compatibility is being used as cover for a second
runtime, mixed ownership, or indefinite legacy writes.

## 4. Warn-only stage

Approve warn-only advancement only when all of the following are true:

- The affected surface already satisfies the compatibility-stage exit criteria.
- Diagnostics are persisted or otherwise attached to the review evidence.
- Warning evaluation blocks zero supported flows by itself.
- Reviewed fixtures or sampled inputs show that warnings match the intended
  future blocking policy.
- Remaining warning classes are named in the review notes together with the
  roadmap item that will retire them.

Do not advance the stage if warnings are being used to defer basic ownership or
schema decisions that should have been settled in compatibility.

## 5. Blocking stage

Approve blocking-stage advancement only when all of the following are true:

- The affected surface satisfies the warn-only exit criteria.
- Deny-path fixtures fail closed in CI with no manual bypass in the evidence
  bundle.
- Rollback steps are documented and return the surface to warn-only behaviour,
  not to inline runtime ownership.
- Any compatibility reads retained for historical records remain available and
  tested where ADR 004 requires them.
- The pull request identifies the legacy path that is now blocked or retired.

Do not advance the stage if the rollback plan depends on reintroducing local
hook execution, inline hosted tool routing, or Corbusier-owned hosted launch
paths.

## 6. End-of-migration gate

Treat migration closure or legacy-path retirement as incomplete until all of
the following are true:

- `docs/roadmap.md`, `docs/corbusier-design.md`, ADR 010, and this checklist
  all describe the same final hosted path.
- The retirement evidence bundle cited by roadmap item `1.6.2` is attached to
  the pull request.
- Legacy write paths are removed only after compatibility reads are no longer
  required or are explicitly retained for historical data.
- `docs/users-guide.md` is updated only when operator-visible behaviour
  changes; otherwise the pull request states that no user-facing workflow
  changed.
