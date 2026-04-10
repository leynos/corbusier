# Ratify the staged migration boundary and phase gates (roadmap 1.1.1)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DRAFT

`PLANS.md` is not present in this repository as of 2026-04-10, so this plan is
the controlling execution document for roadmap item `1.1.1`.

Implementation must not begin until this plan is approved.

## Purpose / big picture

Implement roadmap item `1.1.1` so Corbusier has one ratified migration boundary
for Podbot-hosted execution and one explicit set of phase gates for warn-only,
compatibility, and blocking rollout stages.

After this change, a maintainer or reviewer can:

- read ADRs 001 through 005 and see one consistent account of Podbot
  dependencies, ownership boundaries, and migration sequencing;
- determine from repository-facing documentation exactly what evidence is
  required to enter or leave the warn-only, compatibility, and blocking
  migration phases; and
- review migration pull requests against a stable checklist instead of
  re-litigating whether Corbusier may retain inline runtime ownership.

Observable success means:

- ADRs 001 through 005 cite compatible Podbot dependencies and no longer carry
  contradictory ownership or migration text;
- the repository contains a stable, reviewer-facing description of phase entry
  and exit gates;
- the implementation records which phases require accepted ADRs before merge
  and which documentation updates gate the end of migration; and
- the relevant documentation quality gates pass before roadmap item `1.1.1` is
  marked done.

## Constraints

- Scope is limited to roadmap item `1.1.1`:
  - ratify ADRs 001 through 005 together;
  - define the migration boundary and phase gates;
  - record reviewer-facing advancement criteria and completion gates;
  - do not implement the Podbot adapter seam itself, workspace runtime records,
    wire provisioning, hook control, durable runtime state, or prompt
    validation behaviour from later roadmap items.
- Preserve the migration ownership boundary already proposed by ADR 001 and the
  conformance design:
  - Podbot owns hosted runtime mechanics;
  - Corbusier owns policy, registry, orchestration, durable state, and audit
    interpretation;
  - this task must not reintroduce inline hosted runtime ownership in
    Corbusier's normative text.
- Ratification must be bundle-based:
  - ADRs 001 through 005 must be reviewed and updated as one coherent set;
  - no single ADR may be accepted or rewritten in a way that leaves the other
    foundational ADRs inconsistent.
- Repository-facing documentation for this task must live in stable documents,
  not only in pull-request discussion:
  - update the relevant ADRs;
  - update `docs/adr-010-migration-and-coexistence-strategy.md` with the
    ratified phase-gate model;
  - update `docs/corbusier-design.md` with any binding governance decisions
    that later implementation work depends on;
  - add or update a maintainer-facing checklist document for migration review.
- Respect documentation rules:
  - use en-GB-oxendict spelling;
  - wrap paragraphs to 80 columns;
  - keep the plan and any new checklist file under the repository's 400-line
    limit by splitting if necessary.
- `docs/users-guide.md` should only change if the implementation introduces a
  user-visible operator workflow or migration-state behaviour that operators
  must follow. If the outcome is maintainer-only governance, record explicitly
  that no user-facing guide change was required.
- Mark roadmap item `1.1.1` done in `docs/roadmap.md` only after all planned
  documentation updates and validation gates pass.
- Testing and validation expectations for this item are documentation-first:
  - run Markdown/documentation quality gates unconditionally;
  - if implementation introduces any automation to validate ADR dependency
    consistency or checklist coverage, cover that automation with `rstest`
    unit tests and `rstest-bdd` scenarios only if a user-visible command or
    workflow is added;
  - do not invent a runtime feature solely to satisfy test-count metrics.

## Tolerances (exception triggers)

- Scope: stop and escalate if ratifying `1.1.1` appears to require any
  implementation work from roadmap items `1.1.2` through `1.6.x`.
- ADR churn: stop and escalate if bundle ratification would require reopening
  ADRs 006 through 010 beyond narrow cross-reference updates.
- Review surface: stop and escalate if the change grows beyond roughly 10
  documentation files or 1,500 net lines; split the work or reduce scope.
- Governance ambiguity: stop and escalate if the repository cannot answer which
  phases require accepted ADRs before merge without a separate RFC-level
  decision.
- Validation scope: stop and escalate if meaningful enforcement requires a new
  repository-wide lint or CI subsystem rather than focused documentation and
  checklist updates.
- User-facing impact: stop and escalate if the phase-gate decision forces new
  operator commands, server responses, or UI states that belong in a later
  implementation roadmap item.

## Risks

- Risk: ADRs 001 through 005 use overlapping but not identical dependency lists
  and migration wording, so ratifying one in isolation could harden
  contradictions into the design set. Severity: high. Likelihood: high.
  Mitigation: build an explicit cross-ADR dependency and ownership matrix
  before editing prose.
- Risk: migration phases can remain too qualitative, causing reviewers to make
  inconsistent merge decisions. Severity: high. Likelihood: medium. Mitigation:
  define entry and exit gates in terms of evidence, documents, and specific
  upstream Podbot capabilities.
- Risk: "warn-only", "compatibility", and "blocking" can be interpreted as
  implementation toggles rather than repository governance states. Severity:
  medium. Likelihood: medium. Mitigation: distinguish rollout states, merge
  gates, and runtime behaviour explicitly in ADR 010 and the checklist.
- Risk: no dedicated migration review checklist exists today, so phase-gate
  criteria can end up scattered across ADRs only. Severity: medium. Likelihood:
  high. Mitigation: add one concise checklist document and make the ADRs point
  to it.
- Risk: documentation-only work can miss the requirement to update the design
  document and roadmap because no code changed. Severity: medium. Likelihood:
  medium. Mitigation: treat design and roadmap updates as completion gates, not
  optional follow-up.

## Progress

- [x] (2026-04-10 00:00Z) Reviewed roadmap item `1.1.1`, ADRs 001 through 005,
  ADR 010, the Podbot conformance design, the existing ExecPlan conventions,
  and the repository documentation rules.
- [x] (2026-04-10 00:00Z) Confirmed the upstream Podbot roadmap dependencies
  currently referenced by this phase, including Steps 1.4, 4.5, 4.6, 4.7, 4.8,
  4.9, 4.10, and the later conformance suites.
- [x] (2026-04-10 00:00Z) Authored the initial ExecPlan draft in this file.
- [ ] Await user approval before implementation.
- [ ] Execute stages A through E and keep this section current.

## Surprises & Discoveries

- Observation: roadmap item `1.1.1` names only Podbot Step 1.4 and Step 4.5 as
  required inputs, but ADR 001 currently also cites Step 4.3b and Step 4.6.
  Impact: ratification must decide whether those extra dependencies are truly
  foundational or belong in later implementation items such as `1.1.2` or
  `1.3.3`.
- Observation: ADR 010 already defines a staged migration sequence, but it
  does not yet spell out repository-facing entry and exit gates for warn-only,
  compatibility, and blocking stages. Impact: `1.1.1` must extend ADR 010
  rather than duplicate it elsewhere.
- Observation: the conformance design already recommends explicit phase gates
  and a feature-flagged compatibility bridge for hosted hooks. Impact: `1.1.1`
  should harvest those governance ideas into normative repository documents
  instead of leaving them design-only.
- Observation: no dedicated migration review checklist file currently exists.
  Impact: implementation should create one concise companion document or
  clearly extend an existing maintainer-facing document.

## Decision Log

- Decision: treat `1.1.1` as a governance-and-documentation feature, not as an
  adapter or runtime implementation task. Rationale: the roadmap text asks for
  ratification, advancement criteria, and review gates before downstream
  migration work lands. Date/Author: 2026-04-10 / plan author.
- Decision: ratify ADRs 001 through 005 as a bundle and reconcile them against
  ADR 010 in the same change. Rationale: foundational runtime, workspace, wire,
  source-taxonomy, and hook decisions are mutually dependent, so partial
  ratification risks contradictory ownership or dependency text. Date/Author:
  2026-04-10 / plan author.
- Decision: store the normative phase model in ADR 010 and use one concise
  reviewer checklist document as the operational companion. Rationale: ADR 010
  already owns migration sequencing, while reviewers need a short checklist
  rather than re-reading every ADR on each change. Date/Author: 2026-04-10 /
  plan author.
- Decision: avoid adding new code-level enforcement unless document-only
  governance proves insufficient. Rationale: the roadmap item is about
  ratifying boundaries and gates, so repository-wide automation would be
  additive only if a narrow enforcement gap remains after the docs update.
  Date/Author: 2026-04-10 / plan author.

## Outcomes & Retrospective

Initial planning outcome: the repository now has an implementation sequence for
ratifying the migration boundary, consolidating foundational ADR text, and
turning migration stages into explicit repository review gates. Final outcomes,
deviations, and lessons learned will be recorded after execution.

## Context and orientation

This roadmap item sits at the front of the Podbot conformance phase and governs
how later items are allowed to land.

Primary source documents:

- `docs/roadmap.md` item `1.1.1`
- `docs/adr-001-runtime-boundary-between-corbusier-and-podbot.md`
- `docs/adr-002-workspace-runtime-model-and-source-policy.md`
- `docs/adr-003-mcp-wire-model-and-tool-plane-ownership.md`
- `docs/adr-004-canonical-mcp-source-taxonomy-and-legacy-transport-migration.md`
- `docs/adr-005-hook-execution-contract-and-control-channel-semantics.md`
- `docs/adr-010-migration-and-coexistence-strategy.md`
- `docs/podbot-conformance-design-for-agents-mcp-wires-and-hooks.md`
- upstream Podbot roadmap:
  `https://raw.githubusercontent.com/leynos/podbot/refs/heads/main/docs/podbot-roadmap.md`

Current repository gaps this plan must close:

- ADRs 001 through 005 are all still `Proposed` and have not yet been ratified
  as a coordinated set.
- ADR 010 defines migration phases, but not the repository-facing review gates
  and checklist language requested by roadmap item `1.1.1`.
- The repository lacks a stable migration review checklist covering when
  warn-only, compatibility, and blocking phases may begin and end.
- The design document does not yet act as a consolidated explanation of these
  migration governance boundaries for later implementation work.

## Plan of work

### Stage A: baseline inventory and contradiction matrix

Build the review baseline before changing prose:

- extract the current dependency lists, ownership statements, migration phases,
  and outstanding decisions from ADRs 001 through 005 and ADR 010;
- compare those statements against roadmap item `1.1.1` and the Podbot
  conformance design;
- record every contradiction or ambiguity that affects:
  - Podbot dependency references;
  - whether Corbusier may retain inline runtime ownership;
  - which phases require accepted ADRs before merge; and
  - which documents gate the end of migration.

Go/no-go: do not edit the ADR bundle until the contradiction matrix is explicit
enough that every planned text change can be traced back to a specific mismatch
or missing gate.

### Stage B: ratify ADRs 001 through 005 as one foundational bundle

Update the foundational ADR set coherently:

- normalize Podbot dependency references so the ADR bundle cites consistent
  upstream steps and distinguishes foundational dependencies from later
  delivery dependencies;
- align ownership language so every ADR clearly states that Podbot owns hosted
  runtime mechanics and Corbusier owns policy, registry, orchestration, durable
  state, and audit interpretation;
- reconcile each ADR's migration-plan phases with ADR 010's staged migration
  model;
- decide and document the ratification rule for this cluster:
  - whether these ADRs move to `Accepted` together now; or
  - whether they remain `Proposed` but become required accepted inputs before
    specified later phases can merge;
- update cross-links and companion references so the ADR bundle reads as one
  consistent foundation.

Go/no-go: do not proceed until ADRs 001 through 005 can be read in sequence
without contradictory dependency or ownership text.

### Stage C: codify migration phases and reviewer gates

Turn the staged migration idea into normative repository guidance:

- extend ADR 010 so warn-only, compatibility, and blocking are defined as
  explicit migration stages with entry criteria, exit criteria, rollback
  expectations, and review evidence;
- resolve the roadmap's open governance questions:
  - which phases require accepted ADRs before merge; and
  - which roadmap, design, or checklist updates gate the end of migration;
- add a concise reviewer checklist document, likely under `docs/execplans/` or
  another stable maintainer-facing location, that reviewers can use during
  pull-request review;
- ensure the checklist makes it impossible to advance a phase while reviving
  inline runtime ownership in Corbusier.

Go/no-go: do not treat this stage as complete until a reviewer can determine
phase advancement rules without inferring them from scattered ADR prose.

### Stage D: synchronize roadmap and design guidance

Propagate the ratified governance model into the repository's high-level docs:

- update `docs/corbusier-design.md` with the binding migration boundary and the
  meaning of warn-only, compatibility, and blocking gates for later phase-1
  implementation work;
- update `docs/roadmap.md` only as needed to reflect the ratified gate wording
  or companion-document references, while leaving completion boxes unchecked
  until the implementation is finished;
- assess `docs/users-guide.md` for user-visible operator impact:
  - if none, record in the implementation notes that no user-facing guide
    change was required;
  - if migration-state behaviour becomes operator-visible, document it there in
    a focused way.

Go/no-go: do not mark roadmap item `1.1.1` done until the design document and
any required companion docs reflect the final governance decision.

### Stage E: validation and completion evidence

Validate the documentation change and capture evidence:

- run `make fmt` if Markdown formatting adjustments are needed;
- run `make markdownlint`;
- run `make nixie`;
- if implementation added any automation or code, run the relevant Rust gates
  and tests, including `rstest` and `rstest-bdd` coverage where applicable;
- update this ExecPlan's `Progress`, `Decision Log`, and `Outcomes &
  Retrospective` sections with the final evidence and any deviations;
- only then mark roadmap item `1.1.1` done.

Expected command pattern for gate execution:

```bash
set -o pipefail; make fmt 2>&1 | tee /tmp/1-1-1-fmt.log
set -o pipefail; make markdownlint 2>&1 | tee /tmp/1-1-1-markdownlint.log
set -o pipefail; make nixie 2>&1 | tee /tmp/1-1-1-nixie.log
```

If code or automation is added during implementation, append the relevant
`make check-fmt`, `make lint`, and `make test` invocations with the same
`tee`-captured pattern before completion.
