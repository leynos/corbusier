# Associate branches and pull requests with tasks (roadmap 1.2.2)

This ExecPlan is a living document. The sections `Constraints`, `Tolerances`,
`Risks`, `Progress`, `Surprises & discoveries`, `Decision log`, and
`Outcomes & retrospective` must be kept up to date as work proceeds.

Status: COMPLETE

`PLANS.md` is not present in this repository as of 2026-02-11, so this plan is
the controlling execution document for this feature.

This plan resides at
`docs/execplans/1-2-2-associate-branches-and-pull-requests-with-tasks.md`.

## Purpose / big picture

Implement roadmap item 1.2.2 so that Corbusier can record which Git branch and
which pull request (PR) are associated with a task, retrieve tasks by those
references, and transition task state when a pull request is linked. This
extends the task lifecycle (built in 1.2.1) from issue-origin tracking to full
branch-and-PR tracking.

After this change, an implementer can:

- Create a task from an issue (1.2.1, already works).
- Associate a branch reference with that task and retrieve the task by branch.
- Associate a pull request reference with that task, automatically
  transitioning the task to `InReview`, and retrieve the task by PR reference.
- Query all tasks linked to a given branch (many-to-many: multiple tasks may
  share a single branch).

Success is observable in unit tests, behaviour tests (rstest-bdd), and
PostgreSQL integration tests proving happy and unhappy paths.

## Constraints

- Keep strict hexagonal boundaries:
  - Domain logic must not depend on Diesel, PostgreSQL, or transport details.
  - Ports are defined in the core module and implemented only by adapters.
  - Adapters do not call each other directly.
- Preserve all existing 1.2.1 behaviour and public interfaces.
- Use typed domain errors (`thiserror`) for inspectable failures.
- Use `rstest` for unit/integration fixtures and `rstest-bdd` v0.5.0 for
  behavioural tests.
- Use `pg-embed-setup-unpriv` v0.5.0 for local PostgreSQL-backed tests.
- Maintain repository quality bars:
  - No clippy warnings.
  - Formatting and lint checks pass.
  - Full workspace tests pass.
  - `make markdownlint` and `make nixie` pass.
- Each Rust module touched/added must include module-level `//!` docs.
- Keep file sizes manageable; split modules before crossing the 400-line rule.
- Update docs when behaviour changes:
  `docs/corbusier-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`.
- Use en-GB-oxendict spelling in comments and documentation.
- The relationship between tasks and branches is many-to-many: multiple tasks
  may contribute to a single branch (per the roadmap requirement). The
  relationship between tasks and pull requests follows the same model.
- Each individual task has at most one active branch and at most one open pull
  request at any given time (per design doc F-002-RQ-002 and F-002-RQ-004).
- Do not implement version control system (VCS) API integration (branch
  creation, PR creation, naming policies). This plan covers persistence and
  association only; VCS operations are a separate concern for a future roadmap
  item.

## Tolerances (exception triggers)

- Scope: if implementation exceeds 30 files or 2,500 net lines, stop and
  escalate with a reduced-scope option.
- Interfaces: if implementing 1.2.2 requires changing unrelated public APIs
  outside the task lifecycle boundary, stop and escalate.
- Dependencies: if any new crate beyond those already in Cargo.toml is needed,
  stop and escalate with rationale.
- Iterations: if the same failing test remains unresolved after 3 focused
  attempts, stop and record alternatives.
- Timebox: if any single stage exceeds 4 hours of active work, stop and report
  remaining unknowns.
- Ambiguity: if cardinality rules or state transition semantics are materially
  unclear, stop and request a decision.

## Risks

- Risk: The existing `branch_ref` and `pull_request_ref` columns are
  `VARCHAR(255)`. A structured reference (provider + repository + identifier)
  must fit within that limit and be unambiguously parseable. Git branch names
  may contain `/` characters, making naive delimiter-based parsing ambiguous.
  Severity: medium. Likelihood: medium. Mitigation: use `:` as the delimiter in
  the canonical string format (`provider:owner/repo:branch-name`). Colons are
  forbidden in Git ref names by `git-check-ref-format`, so this is unambiguous.
  Validate this invariant in the `BranchName` constructor.

- Risk: The `row_to_task` function in the Postgres adapter currently has a
  `debug_assert!` that branch/PR fields are `None`. Removing this may mask data
  integrity issues during transition. Severity: low. Likelihood: low.
  Mitigation: replace the blanket assert with a workspace-id-only assert
  (workspace_id is still deferred to 1.2.3). Add parsing validation for the
  branch/PR fields instead.

- Risk: State transition on PR association (task → InReview) may conflict with
  1.2.3's scope ("Enforce task state transitions with validation"). Severity:
  medium. Likelihood: medium. Mitigation: implement the state update as a
  simple field write in 1.2.2 without enforcing transition guards. 1.2.3 will
  add guard logic. Document this decision clearly.

- Risk: Behaviour tests may become brittle if they assert internal
  representation rather than observable behaviour. Severity: medium.
  Likelihood: medium. Mitigation: keep `Then` assertions focused on task
  retrieval outcomes and error surfaces, not internal field representations.

## Progress

- [x] (2026-02-11 00:00Z) Gathered roadmap and design requirements from
  `docs/roadmap.md` and `docs/corbusier-design.md` (§2.2.2, §4.1.1.2).
- [x] (2026-02-11 00:00Z) Mapped existing architecture, domain model, and test
  patterns from the 1.2.1 implementation.
- [x] (2026-02-11 00:00Z) Authored initial ExecPlan draft for roadmap 1.2.2.
- [x] (2026-02-11) Await user approval of this ExecPlan before implementation.
- [x] (2026-02-11) Stage A: domain value objects and error extensions.
- [x] (2026-02-11) Stage B+C: task aggregate, repository port, and adapter
  implementations (Postgres and in-memory).
- [x] (2026-02-11) Stage D: service layer and request types.
- [x] (2026-02-12) Stage E: tests (unit, BDD, integration). 48 new tests,
  435 total, all passing.
- [x] (2026-02-12) Stage F: documentation updates and roadmap closure.

## Surprises & discoveries

- Clippy's `expect_used` deny lint applies to standalone helper functions but
  is automatically allowed inside `#[test]`-attributed functions. Test helpers
  that call `.expect()` must either be inlined into test functions or return
  `Result` and use `?` in callers.
- Stages B and C were merged into a single implementation pass because the
  adapter stubs required by Stage B's go/no-go checkpoint were trivial and
  completing them immediately was more efficient than maintaining `todo!()`
  placeholders.
- The `doc_markdown` clippy lint requires backticking proper nouns like
  `PostgreSQL` in Rust doc comments; the `private_intra_doc_links` lint
  prohibits linking to private constants in public doc comments.

## Decision log

- Decision: Use the existing `branch_ref VARCHAR(255)` and
  `pull_request_ref VARCHAR(255)` columns on the `tasks` table rather than
  introducing join tables. Rationale: the columns were explicitly reserved for
  this purpose in the 1.2.1 implementation decision ("Keep branch, pull
  request, and workspace references nullable in 1.2.1 so roadmap items 1.2.2
  and 1.2.3 can extend the lifecycle without re-shaping issue-origin records").
  Each task has at most one branch and one PR. The many-to-many relationship
  (multiple tasks sharing a branch) is naturally modelled by allowing the same
  string value across multiple rows without a unique constraint on these
  columns. Date/Author: 2026-02-11 / DevBoxer.

- Decision: Store branch and PR references as canonical strings using `:`
  as delimiter: `provider:owner/repo:branch-name` for branches,
  `provider:owner/repo:42` for pull requests. Rationale: colons are forbidden
  in Git ref names by `git-check-ref-format` and are not valid in GitHub/GitLab
  owner or repository names, making parsing unambiguous. The format fits within
  VARCHAR(255) for any reasonable ref. This avoids the complexity of JSONB
  migration while remaining human-readable and deterministic. Date/Author:
  2026-02-11 / DevBoxer.

- Decision: Do not add unique indexes on `branch_ref` or `pull_request_ref`.
  Rationale: the roadmap explicitly states "multiple tasks may contribute to a
  single branch", requiring a many-to-many relationship. A unique index would
  prevent this. Uniqueness of association is enforced at the domain level (each
  task has at most one branch/PR), not at the database level across tasks.
  Date/Author: 2026-02-11 / DevBoxer.

- Decision: When a pull request is associated with a task, transition the task
  state to `InReview` as a simple field write without transition guards.
  Rationale: the roadmap sub-item says "Map pull request identifiers to task
  state updates". The natural mapping is PR → InReview. Transition validation
  (e.g., rejecting InReview → InReview) is deferred to 1.2.3 which explicitly
  covers "Enforce task state transitions with validation". In 1.2.2 the state
  update is unconditional. Date/Author: 2026-02-11 / DevBoxer.

- Decision: Introduce a `VcsProvider` type alias for `IssueProvider` rather
  than renaming the existing enum. Rationale: `BranchRef` and `PullRequestRef`
  use the same provider concept as `IssueRef` but the name `IssueProvider` is
  misleading for branch/PR contexts. A type alias avoids breaking existing code
  while giving new types a natural name. A full rename can follow as a separate
  refactoring commit. Date/Author: 2026-02-11 / DevBoxer.

- Decision: Add a non-unique index on `branch_ref` for query performance but
  not for uniqueness enforcement. Rationale: `find_by_branch_ref` and
  `find_all_by_branch_ref` need efficient lookups. A B-tree index on the column
  (WHERE NOT NULL) accelerates these queries without constraining cardinality.
  Date/Author: 2026-02-11 / DevBoxer.

## Outcomes & retrospective

All six stages completed successfully. 48 new tests added (435 total), all
passing. All quality gates pass: `make check-fmt`, `make lint`, `make test`,
`make markdownlint`, `make nixie`.

Deviations from the original plan:

- Stages B and C were merged into a single pass for efficiency.
- The `VcsProvider` type alias was implemented as planned but is not yet used
  externally; existing code continues to reference `IssueProvider` directly. A
  separate refactoring pass could introduce this alias more broadly.

Lessons learned:

- The `IssueRef` pattern (validated value object with `from_parts`,
  `to_canonical`, `parse_canonical`) proved highly reusable for `BranchRef` and
  `PullRequestRef`.
- Clippy's strict deny configuration requires careful attention in test
  helpers: `expect_used` is allowed in `#[test]` functions but not in
  standalone helpers.
- Using `:` as the canonical delimiter was validated as safe: colons are
  forbidden in Git ref names by `git-check-ref-format`.

## Context and orientation

### Repository state after 1.2.1

The `task` bounded context lives in `src/task/` and follows hexagonal
architecture:

    src/task/
    ├── domain/
    │   ├── mod.rs          Re-exports all domain types
    │   ├── task.rs         Task aggregate root: TaskState, TaskOrigin, Task,
    │   │                   PersistedTaskData
    │   ├── issue.rs        IssueRef, IssueProvider, ExternalIssue,
    │   │                   ExternalIssueMetadata, IssueSnapshot
    │   ├── ids.rs          TaskId (UUID), IssueNumber, RepositoryFullName
    │   └── error.rs        TaskDomainError, ParseTaskStateError
    ├── ports/
    │   └── repository.rs   TaskRepository trait: store, find_by_id,
    │                       find_by_issue_ref
    ├── adapters/
    │   ├── postgres/
    │   │   ├── repository.rs  PostgresTaskRepository (run_blocking pattern)
    │   │   ├── models.rs      TaskRow, NewTaskRow (already has branch_ref,
    │   │   │                  pull_request_ref as Option<String>)
    │   │   └── schema.rs      Diesel table definition
    │   └── memory/
    │       └── task.rs     InMemoryTaskRepository (HashMap + RwLock)
    ├── services/
    │   └── lifecycle.rs    TaskLifecycleService: create_from_issue,
    │                       find_by_issue_ref; CreateTaskFromIssueRequest
    └── tests/
        ├── domain_tests.rs    rstest unit tests for domain types
        └── service_tests.rs   rstest service-level tests

### Database schema (from migration `2026-02-09-000000_add_tasks_table`)

    CREATE TABLE tasks (
        id UUID PRIMARY KEY,
        origin JSONB NOT NULL,
        branch_ref VARCHAR(255),           -- Reserved for 1.2.2 (always NULL)
        pull_request_ref VARCHAR(255),     -- Reserved for 1.2.2 (always NULL)
        state VARCHAR(50) NOT NULL DEFAULT 'draft',
        workspace_id UUID,                 -- Reserved for 1.2.3
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    );

### Current task domain model (key types)

- `TaskState`: Draft, InProgress, InReview, Paused, Done, Abandoned.
- `TaskOrigin`: tagged enum with `Issue { issue_ref, metadata }` variant.
- `Task`: `{ id, origin, state, created_at, updated_at }` — no branch or PR
  fields yet.
- `PersistedTaskData`: parameter object for reconstructing Task from storage.

### Test infrastructure

- Unit tests: `rstest` fixtures in `src/task/tests/`.
- BDD tests: `rstest-bdd` with `.feature` files in `tests/features/`, step
  definitions in `tests/task_issue_steps/`.
- In-memory integration: `tests/in_memory/task_lifecycle_tests.rs`.
- PostgreSQL integration: `tests/postgres/task_lifecycle_tests.rs` using
  `TemporaryDatabase` from `pg-embed-setup-unpriv`.
- Build commands: `make check-fmt`, `make lint`, `make test`.

### Key existing patterns to reuse

- `IssueRef` pattern (`src/task/domain/issue.rs`): validated value object with
  `from_parts()` constructor, `Display`, `TryFrom`, `Serialize/Deserialize`.
- `IssueNumber` pattern (`src/task/domain/ids.rs`): positive integer newtype
  with schema-bound validation.
- `RepositoryFullName` pattern: string newtype with format validation.
- `run_blocking` pattern in Postgres adapter for sync Diesel in async context.
- `to_new_row` / `row_to_task` conversion functions in Postgres adapter.
- BDD world pattern (`tests/task_issue_steps/world.rs`): shared scenario
  state with `run_async` helper.

## Plan of work

### Stage A: domain value objects and error extensions

Create two new domain files following the `IssueRef`/`IssueNumber` pattern.

**New file `src/task/domain/branch.rs`:**

Define `BranchName`, a string newtype that validates: non-empty after trimming,
no colon characters (reserved as delimiter), total length at most 200
characters (leaving room for provider:repo prefix in the 255-char column). Add
`as_str()`, `Display`, `Serialize/Deserialize` (via `serde(transparent)`).

Define `BranchRef`, a composite value object with fields
`provider: IssueProvider` (reuse existing enum),
`repository: RepositoryFullName` (reuse existing type), and
`branch_name: BranchName`. Add a
`from_parts(provider: &str, repository: &str, branch_name: &str)`
`-> Result<Self, TaskDomainError>` constructor that delegates validation to
each component. Add `to_canonical(&self) -> String` returning
`"provider:owner/repo:branch-name"` and
`fn parse_canonical(s: &str) -> Result<Self, TaskDomainError>` for round-trip
parsing. Implement `Display` (delegates to `to_canonical`) and `TryFrom<&str>`
(delegates to `parse_canonical`). Derive
`Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize`.

**New file `src/task/domain/pull_request.rs`:**

Define `PullRequestNumber`, a `u64` newtype with the same validation as
`IssueNumber` (positive, fits `i64::MAX`). Add `value()`, `Display`,
`Serialize/Deserialize`.

Define `PullRequestRef`, a composite value object with fields
`provider: IssueProvider`, `repository: RepositoryFullName`, and
`pull_request_number: PullRequestNumber`. Add `from_parts`, `to_canonical`
(returning `"provider:owner/repo:42"`), `parse_canonical`, `Display`,
`TryFrom<&str>`. Derive same traits as `BranchRef`.

**Modify `src/task/domain/error.rs`:**

Add four new variants to `TaskDomainError`:

- `InvalidBranchName(String)` — branch name is empty, contains colons, or
  exceeds length limit.
- `InvalidPullRequestNumber(u64)` — same validation rules as
  `InvalidIssueNumber`.
- `BranchAlreadyAssociated(TaskId)` — task already has a branch.
- `PullRequestAlreadyAssociated(TaskId)` — task already has a PR.
- `InvalidBranchRefFormat(String)` — canonical string could not be parsed.
- `InvalidPullRequestRefFormat(String)` — canonical string could not be
  parsed.

**Modify `src/task/domain/mod.rs`:**

Add `mod branch; mod pull_request;` and re-export:
`BranchName, BranchRef, PullRequestNumber, PullRequestRef`. Add
`pub type VcsProvider = IssueProvider;`.

Go/no-go: `make check-fmt && make lint` pass. New types compile cleanly.

### Stage B: task aggregate and repository port changes

**Modify `src/task/domain/task.rs`:**

Add optional fields to `Task`:

    branch_ref: Option<BranchRef>,
    pull_request_ref: Option<PullRequestRef>,

Add corresponding fields to `PersistedTaskData`.

Update `new_from_issue` to initialize both as `None`.

Update `from_persisted` to populate from `PersistedTaskData`.

Add accessor methods:

- `pub const fn branch_ref(&self) -> Option<&BranchRef>`
- `pub const fn pull_request_ref(&self) -> Option<&PullRequestRef>`

Add mutation methods:

- `associate_branch(&mut self, BranchRef, &impl Clock)`
  `-> Result<(), TaskDomainError>` — returns `BranchAlreadyAssociated` if
  already set; otherwise sets field and updates `updated_at`.
- `associate_pull_request(&mut self, PullRequestRef, &impl Clock)`
  `-> Result<(), TaskDomainError>` — returns `PullRequestAlreadyAssociated` if
  already set; otherwise sets field, transitions state to `InReview`, and
  updates `updated_at`.

**Modify `src/task/ports/repository.rs`:**

Add three new methods to the `TaskRepository` trait:

- `async fn update(&self, task: &Task) -> TaskRepositoryResult<()>` — persists
  changes to an existing task.
- `async fn find_by_branch_ref(&self, branch_ref: &BranchRef) ->
  TaskRepositoryResult<Vec<Task>>
  ` — returns all tasks linked to the branch (may be multiple due to many-to-many).
- `async fn find_by_pull_request_ref(&self, pr_ref: &PullRequestRef) ->
  TaskRepositoryResult<Vec<Task>>` — returns all tasks linked to the PR.

Note: these return `Vec<Task>` not `Option<Task>` because multiple tasks may
share a branch or PR.

Go/no-go: `make check-fmt && make lint` pass after updating all trait
implementations with `todo!()` stubs.

### Stage C: adapter implementations

**Modify `src/task/adapters/postgres/repository.rs`:**

Update `to_new_row`: serialize `task.branch_ref()` and
`task.pull_request_ref()` using their `to_canonical()` / `to_string()` methods
(replacing the hardcoded `None` values).

Update `row_to_task`: remove the blanket `debug_assert!` for branch/PR. Replace
with workspace-only assert. Parse `branch_ref` and `pull_request_ref` strings
into domain types using `BranchRef::parse_canonical` and
`PullRequestRef::parse_canonical`, mapping parse errors via
`TaskRepositoryError::persistence`.

Implement `update`: use
`diesel::update(tasks::table.filter(tasks::id.eq(...))) .set((...))` to write
`branch_ref`, `pull_request_ref`, `state`, and `updated_at`. Return `NotFound`
if zero rows affected.

Implement `find_by_branch_ref`: filter by `tasks::branch_ref.eq(canonical)`,
collect all matching rows, map each via `row_to_task`.

Implement `find_by_pull_request_ref`: analogous to `find_by_branch_ref`.

**New migration `migrations/2026-02-11-000000_add_branch_pr_lookup_indexes/`:**

up.sql:

    -- Non-unique index for branch reference lookups (many-to-many: multiple
    -- tasks may share a branch).
    CREATE INDEX idx_tasks_branch_ref ON tasks (branch_ref)
        WHERE branch_ref IS NOT NULL;

    -- Non-unique index for pull request reference lookups.
    CREATE INDEX idx_tasks_pull_request_ref ON tasks (pull_request_ref)
        WHERE pull_request_ref IS NOT NULL;

down.sql:

    DROP INDEX IF EXISTS idx_tasks_pull_request_ref;
    DROP INDEX IF EXISTS idx_tasks_branch_ref;

**Modify `src/task/adapters/memory/task.rs`:**

Add reverse indexes to `InMemoryTaskState`:

    branch_index: HashMap<String, Vec<TaskId>>,
    pull_request_index: HashMap<String, Vec<TaskId>>,

(Using `String` canonical form as key, mapping to a Vec of TaskIds for the
many-to-many relationship.)

Update `store` to index branch/PR refs if present.

Implement `update`: replace the stored task, update indexes (remove old
branch/PR mappings, add new ones).

Implement `find_by_branch_ref` and `find_by_pull_request_ref`: look up by
canonical string, return cloned tasks.

**Update `tests/postgres/helpers.rs`:**

Add the new migration SQL constant and apply it in the template setup function.

Go/no-go: `make check-fmt && make lint` pass. Existing tests still pass (1.2.1
behaviour preserved).

### Stage D: service layer

**Modify `src/task/services/lifecycle.rs`:**

Add request types following the `CreateTaskFromIssueRequest` builder pattern:

`AssociateBranchRequest`:

- Fields: `task_id: TaskId`, `provider: String`, `repository: String`,
  `branch_name: String`.
- Constructor: `new(task_id, provider, repository, branch_name)`.

`AssociatePullRequestRequest`:

- Fields: `task_id: TaskId`, `provider: String`, `repository: String`,
  `pull_request_number: u64`.
- Constructor: `new(task_id, provider, repository, pull_request_number)`.

Add service methods to `TaskLifecycleService`:

- `associate_branch(request) -> TaskLifecycleResult<Task>`: validate inputs
  via `BranchRef::from_parts`, fetch task by ID (return NotFound if missing),
  call `task.associate_branch(...)`, call `repository.update(&task)`, return
  updated task.
- `associate_pull_request(request) -> TaskLifecycleResult<Task>`: validate
  inputs via `PullRequestRef::from_parts`, fetch task by ID, call
  `task.associate_pull_request(...)` (which also transitions to InReview), call
  `repository.update(&task)`, return updated task.
- `find_by_branch_ref(branch_ref) -> TaskLifecycleResult<Vec<Task>>`:
  delegate to `repository.find_by_branch_ref`.
- `find_by_pull_request_ref(pr_ref) -> TaskLifecycleResult<Vec<Task>>`:
  delegate to `repository.find_by_pull_request_ref`.

**Modify `src/task/services/mod.rs`:**

Add re-exports for `AssociateBranchRequest` and `AssociatePullRequestRequest`.

Go/no-go: `make check-fmt && make lint` pass.

### Stage E: tests

**Unit tests — extend `src/task/tests/domain_tests.rs` or create new file
`src/task/tests/branch_pr_tests.rs` if domain_tests.rs approaches 400 lines:**

- `BranchName::new` accepts valid names (e.g., `"feature/my-branch"`,
  `"main"`).
- `BranchName::new` rejects empty, whitespace-only, colon-containing, and
  overlength names.
- `BranchRef::from_parts` accepts valid components.
- `BranchRef::from_parts` rejects invalid provider, repository, or branch
  name.
- `BranchRef` round-trips through `to_string()` and `TryFrom<&str>`.
- `PullRequestNumber::new` accepts valid numbers.
- `PullRequestNumber::new` rejects zero and values exceeding `i64::MAX`.
- `PullRequestRef::from_parts` accepts valid components.
- `PullRequestRef` round-trips through `to_string()` and `TryFrom<&str>`.
- `Task::associate_branch` sets branch and updates `updated_at`.
- `Task::associate_branch` rejects when branch already set
  (`BranchAlreadyAssociated`).
- `Task::associate_pull_request` sets PR, transitions to InReview, updates
  `updated_at`.
- `Task::associate_pull_request` rejects when PR already set
  (`PullRequestAlreadyAssociated`).

**Service tests — extend `src/task/tests/service_tests.rs`:**

- `associate_branch` persists and task is retrievable by `find_by_branch_ref`.
- `associate_branch` rejects duplicate branch on same task.
- `associate_branch` returns `NotFound` for unknown task ID.
- `associate_branch` rejects invalid branch name.
- `associate_pull_request` persists, transitions state to InReview, and task
  is retrievable by `find_by_pull_request_ref`.
- `associate_pull_request` rejects duplicate PR on same task.
- `associate_pull_request` returns `NotFound` for unknown task ID.
- Multiple tasks associated with the same branch are all returned by
  `find_by_branch_ref`.

**BDD tests — new file `tests/features/task_branch_pr_association.feature`:**

    Feature: Branch and pull request association with tasks

      Scenario: Associate a branch with a task and retrieve by reference
        Given an external issue "github" "corbusier/core" #200
        And the issue has title "Implement branch tracking"
        And the issue has been converted into a task
        When a branch "github" "corbusier/core" "feature/branch-tracking"
          is associated with the task
        Then the task has an associated branch reference
        And the task can be retrieved by the branch reference

      Scenario: Associate a pull request with a task and verify state
        Given an external issue "github" "corbusier/core" #201
        And the issue has title "Implement PR tracking"
        And the issue has been converted into a task
        When a pull request "github" "corbusier/core" #42
          is associated with the task
        Then the task has an associated pull request reference
        And the task state is in_review

      Scenario: Reject second branch association on the same task
        Given an external issue "github" "corbusier/core" #202
        And the issue has title "Duplicate branch test"
        And the issue has been converted into a task
        And a branch is already associated with the task
        When a second branch is associated with the task
        Then branch association fails with a branch already associated error

      Scenario: Reject second pull request association on the same task
        Given an external issue "github" "corbusier/core" #203
        And the issue has title "Duplicate PR test"
        And the issue has been converted into a task
        And a pull request is already associated with the task
        When a second pull request is associated with the task
        Then pull request association fails with a PR already associated error

**BDD step definitions — new module `tests/task_branch_pr_steps/`:**

Follow the exact pattern of `tests/task_issue_steps/`: `mod.rs`, `world.rs`,
`given.rs`, `when.rs`, `then.rs`. Extend a new world struct
(`TaskBranchPrWorld`) containing the service, pending refs, and result holders.

**BDD runner — new file `tests/task_branch_pr_association_steps.rs`:**

Wire scenarios to the world fixture using `rstest_bdd_macros::scenario`.

**In-memory integration — extend `tests/in_memory/task_lifecycle_tests.rs`:**

- Associate branch, retrieve by branch ref.
- Associate PR, retrieve by PR ref, verify state is InReview.
- Multiple tasks sharing a branch are all returned.

**PostgreSQL integration — extend `tests/postgres/task_lifecycle_tests.rs`:**

- Create task, associate branch, find by ID, verify `branch_ref` is set.
- Create task, associate PR, find by ID, verify `pull_request_ref` and state.
- `find_by_branch_ref` returns matching tasks.
- `find_by_pull_request_ref` returns matching tasks.
- `update` on non-existent task returns `NotFound`.
- Two tasks sharing the same branch are both returned by
  `find_by_branch_ref`.

Go/no-go: `make test` passes across workspace.

### Stage F: documentation and roadmap closure

**Modify `docs/corbusier-design.md`:**

Add implementation decision notes under §2.2.2, after the existing 1.2.1
decision block:

    ###### Implementation Decisions (2026-02-11)

    - Branch and pull request references are stored as canonical string
      representations (`provider:owner/repo:identifier`) in the existing
      `branch_ref` and `pull_request_ref` VARCHAR(255) columns.
    - Multiple tasks may share the same branch reference (many-to-many).
      Each individual task has at most one active branch and at most one
      open pull request.
    - Non-unique partial indexes on `branch_ref` and `pull_request_ref`
      accelerate lookup queries.
    - Associating a pull request transitions the task state to `InReview`.
      State transition validation (guard logic) is deferred to 1.2.3.
    - Domain types `BranchRef` and `PullRequestRef` follow the same
      pattern as `IssueRef` with provider, repository, and identifier
      components.

**Modify `docs/users-guide.md`:**

Add a new section after the existing issue-to-task example, showing:

- How to create an `AssociateBranchRequest` and call `associate_branch`.
- How to create an `AssociatePullRequestRequest` and call
  `associate_pull_request`.
- How to retrieve tasks by branch or PR reference.

**Modify `docs/roadmap.md`:**

Mark 1.2.2 and its three sub-items as done (`[x]`).

**Modify `src/task/mod.rs`:**

Update the module doc comment to reflect 1.2.2 scope alongside 1.2.1.

Go/no-go: `make markdownlint`, `make nixie`, and all quality gates pass.

## Concrete steps

All commands run from repository root: `/home/user/project`.

1. Run baseline quality gates to confirm starting state.

       set -o pipefail; make check-fmt 2>&1 | tee /tmp/1-2-2-baseline-fmt.log
       set -o pipefail; make lint 2>&1 | tee /tmp/1-2-2-baseline-lint.log
       set -o pipefail; make test 2>&1 | tee /tmp/1-2-2-baseline-test.log

   Expected: all exit 0.

2. Create domain value objects (Stage A).

   Create `src/task/domain/branch.rs` and `src/task/domain/pull_request.rs`.
   Extend `error.rs` and `mod.rs`.

       make check-fmt && make lint

   Expected: no errors.

3. Extend task aggregate and repository port (Stage B).

   Modify `task.rs`, `PersistedTaskData`, repository trait.

       make check-fmt && make lint

   Expected: compile errors from unimplemented trait methods (expected;
   adapters need stubs).

4. Implement adapters (Stage C).

   Modify Postgres and in-memory adapters. Create migration. Update helpers.

       make check-fmt && make lint && make test

   Expected: all existing 1.2.1 tests still pass.

5. Implement service layer (Stage D).

       make check-fmt && make lint

   Expected: no errors.

6. Add all tests (Stage E).

       set -o pipefail; make test 2>&1 | tee /tmp/1-2-2-test.log

   Expected: all new and existing tests pass.

7. Update documentation (Stage F).

       set -o pipefail; make markdownlint 2>&1 | tee /tmp/1-2-2-md.log
       set -o pipefail; make nixie 2>&1 | tee /tmp/1-2-2-nixie.log

   Expected: all exit 0.

8. Final quality gate.

       set -o pipefail; make check-fmt 2>&1 | tee /tmp/1-2-2-final-fmt.log
       set -o pipefail; make lint 2>&1 | tee /tmp/1-2-2-final-lint.log
       set -o pipefail; make test 2>&1 | tee /tmp/1-2-2-final-test.log

   Expected: all exit 0.

## Validation and acceptance

Behavioural acceptance criteria:

- Associating a branch with a task succeeds and the task is subsequently
  retrievable by that branch reference.
- Associating a pull request with a task succeeds, transitions the task state
  to InReview, and the task is subsequently retrievable by PR reference.
- Attempting to associate a second branch or PR with a task that already has
  one fails with a typed, auditable domain error.
- Multiple tasks associated with the same branch are all returned when
  querying by that branch reference.
- All existing 1.2.1 behaviour (issue-to-task creation, retrieval by issue
  ref, duplicate rejection) continues to work unchanged.

Test acceptance criteria:

- Unit tests (`rstest`) cover:
  - `BranchName` / `BranchRef` / `PullRequestNumber` / `PullRequestRef`
    validation and round-trip parsing.
  - `Task::associate_branch` and `Task::associate_pull_request` happy and
    unhappy paths.
- Behaviour tests (`rstest-bdd`) cover at least:
  - Branch association and retrieval.
  - PR association with state transition.
  - Duplicate branch rejection.
  - Duplicate PR rejection.
- In-memory integration tests cover service-level flows.
- PostgreSQL integration tests cover persistence round-trips, lookups by
  branch/PR ref, and multi-task branch sharing.

Quality criteria:

- `make check-fmt` passes.
- `make lint` passes with no warnings.
- `make test` passes across workspace.
- `make markdownlint` and `make nixie` pass.

## Idempotence and recovery

- Development/test runs are repeatable:
  - In-memory tests have no external side effects.
  - PostgreSQL tests use temporary databases and cluster guards.
- If migration work fails midway: revert migration files and re-run. The
  migration only adds indexes to existing columns, so up/down is safe.
- If adapter work fails midway: existing 1.2.1 tests provide a safety net.
  All 1.2.1 behaviour must pass before proceeding.

## Artefacts and notes

Files to create:

- `src/task/domain/branch.rs`
- `src/task/domain/pull_request.rs`
- `migrations/2026-02-11-000000_add_branch_pr_lookup_indexes/up.sql`
- `migrations/2026-02-11-000000_add_branch_pr_lookup_indexes/down.sql`
- `tests/features/task_branch_pr_association.feature`
- `tests/task_branch_pr_steps/mod.rs`
- `tests/task_branch_pr_steps/world.rs`
- `tests/task_branch_pr_steps/given.rs`
- `tests/task_branch_pr_steps/when.rs`
- `tests/task_branch_pr_steps/then.rs`
- `tests/task_branch_pr_association_steps.rs`

Files to modify:

- `src/task/domain/error.rs` — add new error variants
- `src/task/domain/mod.rs` — add modules and re-exports
- `src/task/domain/task.rs` — add branch/PR fields and association methods
- `src/task/ports/repository.rs` — add update, find_by_branch_ref,
  find_by_pull_request_ref
- `src/task/adapters/postgres/repository.rs` — implement new methods, update
  row conversions
- `src/task/adapters/memory/task.rs` — implement new methods, add indexes
- `src/task/services/lifecycle.rs` — add request types and service methods
- `src/task/services/mod.rs` — add re-exports
- `src/task/mod.rs` — update module doc
- `src/task/tests/domain_tests.rs` — add value object and aggregate tests
- `src/task/tests/service_tests.rs` — add association service tests
- `tests/in_memory/task_lifecycle_tests.rs` — add integration tests
- `tests/postgres/task_lifecycle_tests.rs` — add integration tests
- `tests/postgres/helpers.rs` — add migration constant
- `docs/corbusier-design.md` — add implementation decision
- `docs/users-guide.md` — add branch/PR association examples
- `docs/roadmap.md` — mark 1.2.2 as done

## Interfaces and dependencies

Prescriptive interfaces for this milestone:

Domain types (all in `src/task/domain/`):

- `BranchName` — string newtype, validated (non-empty, no colons, ≤200 chars).
- `BranchRef` — `{ provider: IssueProvider, repository: RepositoryFullName,
  branch_name: BranchName
  }`. Canonical format: `"provider:owner/repo:branch-name"`.
- `PullRequestNumber` — `u64` newtype, validated (positive, ≤ `i64::MAX`).
- `PullRequestRef` — `{ provider: IssueProvider, repository:
  RepositoryFullName, pull_request_number: PullRequestNumber
  }`. Canonical format: `"provider:owner/repo:42"`.
- `VcsProvider` — type alias for `IssueProvider`.

Port contract extensions (in `src/task/ports/repository.rs`):

- `async fn update(&self, task: &Task) -> TaskRepositoryResult<()>`
- `async fn find_by_branch_ref(&self, branch_ref: &BranchRef) ->
  TaskRepositoryResult<Vec<Task>>`
- `async fn find_by_pull_request_ref(&self, pr_ref: &PullRequestRef) ->
  TaskRepositoryResult<Vec<Task>>`

Service methods (in `src/task/services/lifecycle.rs`):

- `associate_branch(AssociateBranchRequest) -> TaskLifecycleResult<Task>`
- `associate_pull_request(AssociatePullRequestRequest) ->
  TaskLifecycleResult<Task>`
- `find_by_branch_ref(&BranchRef) -> TaskLifecycleResult<Vec<Task>>`
- `find_by_pull_request_ref(&PullRequestRef) ->
  TaskLifecycleResult<Vec<Task>>`

Dependencies: no new crates required. All functionality uses existing
dependencies (`diesel`, `serde`, `chrono`, `uuid`, `mockable`, `thiserror`,
`async-trait`, `rstest`, `rstest-bdd`, `pg-embed-setup-unpriv`).

## Revision note

Initial draft created for roadmap item 1.2.2 based on repository state after
1.2.1 completion and design references dated 2026-02-11.
