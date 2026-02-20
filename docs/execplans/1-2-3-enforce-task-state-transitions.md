# Enforce task state transitions with validation (roadmap 1.2.3)

This Execution Plan (ExecPlan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`,
`Decision log`, and `Outcomes & retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

`PLANS.md` is not present in this repository as of 2026-02-16, so this plan is
the controlling execution document for this feature.

This plan resides at `docs/execplans/1-2-3-enforce-task-state-transitions.md`.

## Purpose / big picture

Implement roadmap item 1.2.3 so that Corbusier enforces a task state machine:
only valid state transitions are permitted, and invalid transitions are
rejected with a typed domain error carrying the task identifier, the current
state, and the requested target state. This error serves as the "auditable
error event" required by the roadmap success criteria.

After this change, a developer can:

- Transition a `Draft` task to `InProgress` via a new `transition_task` service
  method and see it succeed.
- Attempt to transition a `Draft` task directly to `Done` and receive an
  `InvalidStateTransition` error containing the task ID, `from: draft`, and
  `to: done`.
- Observe that `Done` and `Abandoned` are terminal states that reject all
  outgoing transitions.
- Continue to use `associate_pull_request` on a `Draft` task (which transitions
  to `InReview`), but observe that it fails on tasks in terminal states.

Success is observable by running `make all` and seeing all existing tests plus
new unit tests, Behaviour-Driven Development (BDD) scenarios, and integration
tests pass.

## Constraints

- Keep strict hexagonal boundaries:
  - All state machine logic (transition table, validation, `transition_to`
    method) lives in the domain layer (`src/task/domain/`).
  - The service layer orchestrates domain calls and repository persistence
    only.
  - Adapters are not modified for state machine logic.
- Preserve all existing 1.2.1 and 1.2.2 behaviour and public interfaces.
  Existing tests must continue to pass without modification (unless a test was
  asserting incorrect behaviour that the state machine now correctly rejects).
- Use typed domain errors (`thiserror`) for the new `InvalidStateTransition`
  variant, consistent with the existing `TaskDomainError` enum.
- Use `rstest` for unit fixtures and `rstest-bdd` v0.5.0 for behavioural tests.
- Maintain repository quality bars:
  - No clippy warnings (`cargo clippy -- -D warnings`).
  - Formatting passes (`cargo fmt -- --check`).
  - Full workspace tests pass (`cargo nextest run`).
  - `make markdownlint` and `make nixie` pass.
- Each Rust module touched or added must include module-level `//!` docs.
- Keep file sizes below 400 lines; split modules before crossing.
- Update `docs/users-guide.md` with new behaviour and code examples.
- Update `docs/roadmap.md` to mark 1.2.3 as complete.
- Use en-GB-oxendict spelling in comments and documentation.
- The `TaskState` enum's `serde(rename_all = "snake_case")` serialization
  format must not change.
- No new external crate dependencies.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 18 files (net new
  plus modified), stop and escalate.
- Interfaces: if any existing public API signature must change (beyond adding
  new methods or new error variants), stop and escalate.
- Dependencies: if a new external crate is required, stop and escalate.
- Iterations: if the same failing test remains unresolved after 3 focused
  attempts, stop and record alternatives.
- Line budget: if any single file would exceed 380 lines (leaving margin below
  the hard 400-line limit), stop and consider splitting.
- Ambiguity: if the state transition table needs revision in a way that would
  break existing behaviour, stop and present options.

## Risks

- Risk: The existing `associate_pull_request` method unconditionally sets
  state to `InReview` from any state. After adding state machine validation,
  calling `associate_pull_request` on a task in a terminal state (`Done`,
  `Abandoned`) will now fail. No existing test does this, so breakage is
  unlikely. Severity: low. Likelihood: low. Mitigation: search all tests for
  `associate_pull_request` calls and verify the task state at call time is
  always a non-terminal state.

- Risk: The design document (section 6.1.2) shows a `transition_to` method
  signature that includes `TransitionContext`, `DomainEvent`, workspace
  validation, and quality gates. These are future concerns not yet implemented.
  Severity: medium. Likelihood: certain. Mitigation: implement the simpler
  `transition_to` that validates the state machine and updates the timestamp,
  without `TransitionContext` or `DomainEvent`. Document this scoping decision.

- Risk: Adding `Display` to `TaskState` for error formatting could conflict
  with serde or debug formatting. Severity: low. Likelihood: low. Mitigation:
  `Display` delegates to `as_str()` which already produces the canonical string
  form. This is compatible with serde's `rename_all`.

## Progress

- [x] Gather requirements and write ExecPlan draft.
- [x] Stage A: domain layer changes (error variant, state machine methods,
  `transition_to`, update `associate_pull_request`).
- [x] Stage B: service layer changes (`TransitionTaskRequest` DTO,
  `transition_task` method, `InvalidState` error variant, re-exports).
- [x] Stage C: unit tests for state machine (parametric truth table, valid
  and invalid transitions, terminal state tests).
- [x] Stage D: BDD feature file and step definitions for state transitions.
- [x] Stage E: in-memory integration tests for `transition_task` service method.
- [x] Stage F: documentation updates (users guide, module docs, roadmap,
  design doc).
- [x] Stage G: final validation with `make all`.

## Surprises & discoveries

- PostgreSQL integration tests initially failed with:
  `sh: 1: cannot create /dev/null: Permission denied`. The environment had
  `/dev/null` set to mode `0644`; changing to `0666` unblocked embedded
  PostgreSQL startup in tests.
- `pg_embedded_setup_unpriv` is consumed as a dependency in this workspace's
  Postgres test harness, but the workspace does not expose a
  `pg_embedded_setup_unpriv` binary target directly.

## Decision log

- Decision: Allow `Draft` -> `InReview` as a valid transition.
  Rationale: the existing `associate_pull_request` method transitions from
  `Draft` to `InReview`. Multiple unit tests, BDD scenarios, integration tests,
  and the user guide depend on this behaviour. Disallowing it would break
  backward compatibility without clear benefit. A developer may create an issue
  and immediately open a PR without an intermediate `InProgress` step. The
  design document's notation "Draft->InProgress->InReview->Done" describes the
  typical happy path, not the only valid transitions. Date/Author: 2026-02-16 /
  plan author.

- Decision: implement `transition_to` without `TransitionContext` or domain
  events. Rationale: roadmap 1.2.3 requires "define allowed transitions and
  terminal states" and "reject invalid transitions with typed errors". Domain
  events, workspace validation, and richer context belong to future roadmap
  items (2.3.1 hook engine requires 1.2.3 as a prerequisite). The simpler
  interface is sufficient and avoids speculative abstraction. Date/Author:
  2026-02-16 / plan author.

- Decision: place the allowed-transitions lookup in a `const fn` method
  `TaskState::can_transition_to` rather than inlining in `Task::transition_to`.
  Rationale: the state machine definition is a property of `TaskState` itself,
  making it independently testable. `Task::transition_to` handles mutation and
  error wrapping. Date/Author: 2026-02-16 / plan author.

- Decision: validate state transition *before* mutating PR ref in
  `associate_pull_request`. Rationale: no field should be mutated unless all
  validations pass. If the state machine rejects the transition, the PR ref
  must not be set either. Date/Author: 2026-02-16 / plan author.

- Decision: add `ParseTaskStateError` as an `InvalidState` variant on
  `TaskLifecycleError` rather than wrapping it in `TaskDomainError`. Rationale:
  `ParseTaskStateError` is a service-boundary concern (parsing user-supplied
  string to `TaskState`). Keeping it separate from domain validation errors is
  cleaner. The service error enum already has `Domain` and `Repository`
  variants; adding `InvalidState` follows the same pattern. Date/Author:
  2026-02-16 / plan author.

- Decision: no separate Postgres integration tests for state transitions.
  Rationale: the state machine is purely domain logic. The Postgres adapter
  persists the `state` column as a string and round-trips it via
  `TaskState::try_from`. In-memory integration tests and BDD tests provide
  sufficient coverage. Existing Postgres tests for PR association already
  verify state persistence. Date/Author: 2026-02-16 / plan author.

## Outcomes & retrospective

Implemented roadmap item 1.2.3 with domain-enforced task state transitions,
typed invalid-transition errors, service orchestration support, and new unit,
behavioural, and integration coverage.

Validation outcomes:

- `make all` passed (includes `check-fmt`, `lint`, and full workspace tests).
- `make markdownlint` passed.
- `make nixie` passed.
- `cargo nextest run --all-targets --all-features` passed with
  `493 passed, 1 skipped`.

Scope outcome:

- Final implementation touched 19 files (9 new, 10 modified). This exceeded
  the stated 18-file tolerance by one file because roadmap stage F required an
  explicit design-document update in `docs/corbusier-design.md`.

## Context and orientation

Corbusier is a Rust project using hexagonal architecture. The task module
manages the lifecycle of development tasks created from external issue tracker
metadata.

### Key files

- `src/task/domain/task.rs` (254 lines) -- `Task` aggregate root and
  `TaskState` enum. Currently has six states (`Draft`, `InProgress`,
  `InReview`, `Paused`, `Done`, `Abandoned`). The only explicit state
  transition is in `associate_pull_request` (line 232) which unconditionally
  sets state to `InReview` with a comment deferring validation to 1.2.3.
- `src/task/domain/error.rs` (57 lines) -- `TaskDomainError` enum using
  `thiserror::Error`. Also contains `ParseTaskStateError`.
- `src/task/domain/mod.rs` (23 lines) -- re-exports from the domain module.
- `src/task/services/lifecycle.rs` (321 lines) -- `TaskLifecycleService` with
  request DTOs for task creation, branch association, and PR association.
- `src/task/services/mod.rs` (8 lines) -- re-exports `lifecycle` types.
- `src/task/mod.rs` (20 lines) -- module-level doc comment referencing
  roadmap items 1.2.1 and 1.2.2.
- `src/task/tests/mod.rs` (7 lines) -- declares test submodules.
- `src/task/tests/branch_pr_tests.rs` (266 lines) -- domain tests for branch
  and PR value objects and task association. Contains tests that associate a PR
  on a `Draft` task and expect `InReview` (lines 229-243).
- `tests/features/task_branch_pr_association.feature` (33 lines) -- BDD
  feature for PR association.
- `tests/task_branch_pr_association_steps.rs` (42 lines) -- BDD scenario
  runner following `#[scenario(...)]` + `#[tokio::test]` pattern.
- `tests/task_branch_pr_steps/` -- BDD step definitions with `world.rs`
  (world struct), `given.rs`, `when.rs`, `then.rs`.
- `tests/in_memory/task_lifecycle_tests.rs` (309 lines) -- in-memory
  integration tests. Test at line 153 associates a PR on a `Draft` task.
- `docs/users-guide.md` (174 lines) -- user-facing documentation.
- `docs/roadmap.md` -- roadmap with 1.2.3 checkbox items at lines 53-60.

### State machine definition

Table 1. Allowed task state transitions for roadmap item 1.2.3.

| From state   | Allowed target states                     |
| ------------ | ----------------------------------------- |
| `Draft`      | `InProgress`, `InReview`, `Abandoned`     |
| `InProgress` | `InReview`, `Paused`, `Done`, `Abandoned` |
| `InReview`   | `InProgress`, `Done`, `Abandoned`         |
| `Paused`     | `InProgress`, `Abandoned`                 |
| `Done`       | *(terminal -- no outgoing transitions)*   |
| `Abandoned`  | *(terminal -- no outgoing transitions)*   |

Self-transitions (e.g., `Draft` -> `Draft`) are not permitted.

## Plan of work

### Stage A: domain layer changes

Edit `src/task/domain/task.rs`:

A1. Add `Display` implementation for `TaskState` after the existing
`impl TaskState` block (line 42). Delegates to `as_str()`, which is needed for
the `#[error(...)]` format string on the new error variant.

A2. Add two new methods to the existing `impl TaskState` block:

- `can_transition_to(self, target: Self) -> bool` -- a `const fn` that encodes
  the transition table above using a `matches!` expression over all valid
  `(from, to)` pairs.
- `is_terminal(self) -> bool` -- a `const fn` returning `true` for `Done` and
  `Abandoned`.

A3. Add `transition_to(&mut self, target: TaskState, clock: &impl Clock)` to
`impl Task`. This method checks `self.state.can_transition_to(target)`,
returning `Err(TaskDomainError::InvalidStateTransition { ... })` if invalid, or
applying the state change and calling `self.touch(clock)` if valid.

A4. Update `associate_pull_request` to validate the transition before mutating
any fields. Replace the direct `self.state = TaskState::InReview` assignment
with a pre-check using `can_transition_to`, followed by `associate_ref`, then
the state assignment and `touch`. Remove the comment about deferring to 1.2.3.
Update the doc comment's `# Errors` section to mention `InvalidStateTransition`.

Edit `src/task/domain/error.rs`:

A5. Add `InvalidStateTransition` variant to `TaskDomainError`:

    InvalidStateTransition {
        task_id: super::TaskId,
        from: super::TaskState,
        to: super::TaskState,
    }

with error message:
`invalid state transition for task {task_id}: cannot move from {from} to {to}`.

Estimated post-edit line counts: `task.rs` ~310, `error.rs` ~70. Both within
limits.

### Stage B: service layer changes

Edit `src/task/services/lifecycle.rs`:

B1. Add `TransitionTaskRequest` data transfer object (DTO) after
`AssociatePullRequestRequest`. Takes `task_id: TaskId` and
`target_state: String` (string at service boundary, matching the pattern of
other request DTOs). Constructor:
`new(task_id: TaskId, target_state: impl Into<String>)`.

B2. Add `InvalidState` variant to `TaskLifecycleError`:

    #[error(transparent)]
    InvalidState(#[from] ParseTaskStateError),

Add `ParseTaskStateError` to the imports from `crate::task::domain`.

B3. Add `transition_task` method on `TaskLifecycleService`:

    pub async fn transition_task(
        &self,
        request: TransitionTaskRequest,
    ) -> TaskLifecycleResult<Task>

Parses `target_state` via `TaskState::try_from`, looks up the task, calls
`task.transition_to(target, &*self.clock)`, persists with
`self.repository.update(&task)`, returns the updated task.

B4. Update `src/task/services/mod.rs` re-exports to include
`TransitionTaskRequest`.

Estimated post-edit line count for `lifecycle.rs`: ~370. Within limits.

### Stage C: unit tests for state machine

Create `src/task/tests/state_transition_tests.rs`:

C1. Parametric truth table test for `TaskState::can_transition_to` covering all
36 `(from, to)` state pairs using `#[rstest]` with `#[case]`.

C2. Parametric test for `TaskState::is_terminal` covering all 6 states.

C3. Happy-path test: `transition_from_draft_to_in_progress_succeeds` -- creates
a `Draft` task, transitions to `InProgress`, asserts state and `updated_at`
change.

C4. Unhappy-path test: `transition_from_draft_to_done_is_rejected` -- creates a
`Draft` task, attempts `Done`, asserts `InvalidStateTransition` error with
correct fields, asserts state unchanged.

C5. Terminal state test: `terminal_state_rejects_all_transitions` --
parameterized over `Done` and `Abandoned`, navigates to each terminal state,
then asserts all outgoing transitions are rejected.

Register the new module in `src/task/tests/mod.rs`:
`mod state_transition_tests;`.

Estimated file size: ~160 lines.

### Stage D: BDD feature and step definitions

D1. Create `tests/features/task_state_transitions.feature` with three scenarios:

- "Transition a draft task to in progress" (happy path)
- "Reject transition from draft to done" (invalid transition)
- "Reject transition from a terminal state" (terminal state guard)

D2. Create `tests/task_state_transition_steps/` directory with:

- `mod.rs` -- declares `given`, `when`, `then`, and `pub mod world`.
- `world.rs` -- `TaskTransitionWorld` struct holding service, pending issue
  state, `last_created_task`, `last_transition_result`. Follows the exact
  pattern of `tests/task_branch_pr_steps/world.rs`.
- `given.rs` -- reuses the issue creation pattern (external issue, title,
  convert to task). Adds `the task has been transitioned to "{target_state}"`
  step for pre-conditioning.
- `when.rs` -- `the task is transitioned to "{target_state}"` step calling
  `service.transition_task`.
- `then.rs` -- `the task state is {state}` and
  `the transition fails with an invalid state transition error` steps.

D3. Create `tests/task_state_transition_steps.rs` scenario runner with
`#[scenario(...)]` + `#[tokio::test(flavor = "multi_thread")]` for each
scenario.

### Stage E: in-memory integration tests

Add three tests to `tests/in_memory/task_lifecycle_tests.rs`:

E1. `transition_task_from_draft_to_in_progress` -- creates task, transitions
via service, asserts `InProgress` state.

E2. `transition_rejects_invalid_state_change` -- creates task, attempts `Draft`
-> `Done`, asserts `InvalidStateTransition` error.

E3. `transition_rejects_unknown_state_string` -- creates task, attempts
transition to `"nonexistent_state"`, asserts `InvalidState` error.

Add `TransitionTaskRequest` and `TaskState` to the import block.

Estimated post-edit line count: ~365 lines. Within limits.

### Stage F: documentation updates

F1. Update `src/task/mod.rs` module doc comment to reference roadmap 1.2.3.

F2. Add "Task state transitions" section to `docs/users-guide.md` after "Branch
and pull request association". Include a code example showing valid transition,
invalid transition (error), and the state machine table.

F3. Update `docs/roadmap.md` lines 53-60: change `- [ ] 1.2.3` to `- [x] 1.2.3`
and all three sub-checkboxes to `[x]`.

F4. Record design decisions in `docs/corbusier-design.md` noting that 1.2.3
uses `can_transition_to` predicate on `TaskState` and `InvalidStateTransition`
error variant, without `TransitionContext` or domain events at this stage.

### Stage G: final validation

Run `make all` (which executes `check-fmt`, `lint`, and `test`). Verify zero
failures. Fix any issues discovered.

## Concrete steps

All commands run from `/home/user/project`.

1. Write the execplan file to
   `docs/execplans/1-2-3-enforce-task-state-transitions.md`.
2. Edit `src/task/domain/error.rs` -- add `InvalidStateTransition` variant
   (Stage A5).
3. Edit `src/task/domain/task.rs` -- add `Display` impl (A1),
   `can_transition_to` and `is_terminal` (A2), `transition_to` (A3), update
   `associate_pull_request` (A4).
4. Run `cargo check --workspace` to verify compilation.
5. Edit `src/task/services/lifecycle.rs` -- add `TransitionTaskRequest` (B1),
   `InvalidState` variant (B2), `transition_task` method (B3).
6. Edit `src/task/services/mod.rs` -- add `TransitionTaskRequest` to
   re-exports (B4).
7. Run `cargo check --workspace` to verify compilation.
8. Create `src/task/tests/state_transition_tests.rs` (Stage C).
9. Edit `src/task/tests/mod.rs` -- add `mod state_transition_tests;`.
10. Run
    `set -o pipefail && cargo nextest run --workspace 2>&1 | tee /tmp/test-run-1.log`.
11. Create `tests/features/task_state_transitions.feature` (D1).
12. Create `tests/task_state_transition_steps/` with `mod.rs`, `world.rs`,
    `given.rs`, `when.rs`, `then.rs` (D2).
13. Create `tests/task_state_transition_steps.rs` scenario runner (D3).
14. Add integration tests to `tests/in_memory/task_lifecycle_tests.rs` (E1-E3).
15. Run
    `set -o pipefail && cargo nextest run --workspace 2>&1 | tee /tmp/test-run-2.log`.
16. Update documentation files (F1-F4).
17. Run `set -o pipefail && make all 2>&1 | tee /tmp/make-all.log`.
18. Review output and address any failures.

## Validation and acceptance

Quality criteria (what "done" means):

- Tests: `cargo nextest run --workspace --all-targets --all-features` passes
  with zero failures. The parametric test `can_transition_to_returns_expected`
  covers all 36 `(from, to)` state pairs. The three BDD scenarios pass. All
  existing tests continue to pass.
- Lint/typecheck:
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  produces zero warnings.
- Formatting: `cargo fmt --all -- --check` reports no changes needed.
- Documentation: `cargo doc --no-deps` builds without warnings.

Quality method (verification approach):

    make all

Expected output includes a test summary line with zero failures.

Behavioural acceptance:

1. A `Draft` task transitions to `InProgress` via `transition_task`.
2. A `Draft` task cannot transition to `Done` -- the error contains the task
   ID, `from: draft`, `to: done`.
3. A `Done` task cannot transition to any state.
4. `associate_pull_request` on a `Draft` task still succeeds and transitions
   to `InReview`.
5. `associate_pull_request` on a `Done` task fails with
   `InvalidStateTransition`.

## Idempotence and recovery

All steps are safe to re-run. Test runs are inherently idempotent. File edits
are additive (new methods, new variants, new files). If a step fails partway,
re-running `make all` from the repository root will identify remaining issues.

## Artifacts and notes

### New files (9)

Table 2. New files added for this implementation.

| File                                                     | Purpose                      |
| -------------------------------------------------------- | ---------------------------- |
| `docs/execplans/1-2-3-enforce-task-state-transitions.md` | This ExecPlan                |
| `src/task/tests/state_transition_tests.rs`               | Unit tests for state machine |
| `tests/features/task_state_transitions.feature`          | BDD feature file             |
| `tests/task_state_transition_steps.rs`                   | BDD scenario runner          |
| `tests/task_state_transition_steps/mod.rs`               | Step definitions module      |
| `tests/task_state_transition_steps/world.rs`             | BDD world struct             |
| `tests/task_state_transition_steps/given.rs`             | Given step definitions       |
| `tests/task_state_transition_steps/when.rs`              | When step definitions        |
| `tests/task_state_transition_steps/then.rs`              | Then step definitions        |

### Modified files (10)

Table 3. Existing files modified for this implementation.

| File                                      | Change                                                                                              |
| ----------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `src/task/domain/error.rs`                | Add `InvalidStateTransition` variant                                                                |
| `src/task/domain/task.rs`                 | Add `Display`, `can_transition_to`, `is_terminal`, `transition_to`; update `associate_pull_request` |
| `src/task/services/lifecycle.rs`          | Add `TransitionTaskRequest`, `transition_task`, `InvalidState` variant                              |
| `src/task/services/mod.rs`                | Add `TransitionTaskRequest` to re-exports                                                           |
| `src/task/tests/mod.rs`                   | Add `mod state_transition_tests;`                                                                   |
| `src/task/mod.rs`                         | Update module doc comment                                                                           |
| `tests/in_memory/task_lifecycle_tests.rs` | Add state transition integration tests                                                              |
| `docs/users-guide.md`                     | Add "Task state transitions" section                                                                |
| `docs/roadmap.md`                         | Mark 1.2.3 as complete                                                                              |
| `docs/corbusier-design.md`                | Record 1.2.3 implementation decisions                                                               |

Total: 19 files (9 new + 10 modified).

## Interfaces and dependencies

No new external dependencies. All functionality uses existing crates:
`thiserror`, `mockable`, `chrono`, `serde`, `rstest`, `rstest-bdd-macros`,
`eyre`.

### New public interfaces

In `src/task/domain/task.rs`:

    impl std::fmt::Display for TaskState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
    }

    impl TaskState {
        pub const fn can_transition_to(self, target: Self) -> bool;
        pub const fn is_terminal(self) -> bool;
    }

    impl Task {
        pub fn transition_to(
            &mut self,
            target: TaskState,
            clock: &impl Clock,
        ) -> Result<(), TaskDomainError>;
    }

In `src/task/domain/error.rs`:

    pub enum TaskDomainError {
        // … existing variants …
        InvalidStateTransition {
            task_id: super::TaskId,
            from: super::TaskState,
            to: super::TaskState,
        },
    }

In `src/task/services/lifecycle.rs`:

    pub struct TransitionTaskRequest { .. }

    impl TransitionTaskRequest {
        pub fn new(task_id: TaskId, target_state: impl Into<String>) -> Self;
    }

    pub enum TaskLifecycleError {
        // … existing variants …
        InvalidState(#[from] ParseTaskStateError),
    }

    impl<R, C> TaskLifecycleService<R, C> {
        pub async fn transition_task(
            &self,
            request: TransitionTaskRequest,
        ) -> TaskLifecycleResult<Task>;
    }
