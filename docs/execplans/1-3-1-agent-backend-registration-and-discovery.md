# Implement agent backend registration and discovery (Roadmap 1.3.1)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Corbusier is an AI agent orchestration platform. Before it can route work to
different AI backends (Claude Code SDK, Codex CLI, future providers), it needs
a registry where backends declare their identity and capabilities. This plan
adds a new `agent_backend` subsystem that lets callers register agent backends,
look them up by ID or name, list all registered backends, and deregister them.

After this change, a caller can:

1. Register two distinct backends (e.g. "claude_code_sdk" and "codex_cli")
   with capability metadata.
2. List all registered backends and confirm both appear.
3. Look up a backend by name or ID.
4. Deregister a backend and confirm it no longer appears in active listings
   (inactive backends still appear in `list_all`).

Observable success: `make all` passes, including new unit tests, in-memory
integration tests, Behaviour-Driven Development (BDD) feature tests, and
Postgres integration tests that
exercise the full registration-and-discovery lifecycle.

## Constraints

- The hexagonal architecture must be preserved: domain types must have zero
  infrastructure imports; ports define trait contracts; adapters implement them.
- All existing tests must continue to pass. No regressions.
- Public interfaces of the `message` and `task` modules must not change.
- Rust edition 2024. All Clippy warnings denied (`-D warnings`).
- No `unsafe` code.
- Max 400 lines per file.
- Module-level doc comments (`//!`) required on every module.
- Rustdoc comments (`///`) required on every public item.
- Commit gating: `make check-fmt && make lint && make test` must pass before
  each commit.
- The new migration must be additive (no modifications to existing tables).
- The Postgres test template helper (`tests/postgres/helpers.rs`) must include
  the new migration SQL so all Postgres tests run against the full schema.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 35 files (net) or
  1500 lines of new code, stop and escalate.
- Interface: if any existing public API signature must change, stop and
  escalate.
- Dependencies: if a new external crate dependency is required beyond what is
  already in `Cargo.toml`, stop and escalate.
- Iterations: if tests still fail after 5 fix-and-rerun cycles in any single
  milestone, stop and escalate.
- Ambiguity: if multiple valid interpretations exist and the choice materially
  affects downstream roadmap items (1.3.2, 1.3.3), stop and present options.

## Risks

- Risk: The design document does not define concrete `AgentCapabilities` or
  `BackendInfo` struct fields — they must be inferred from the trait signature and
  the table of known backends (Claude Code SDK, Codex CLI).
  Severity: low
  Likelihood: high
  Mitigation: Define minimal, extensible structs with JSONB-backed flexible
  fields. The capabilities struct captures booleans for known features
  (streaming, tool calls) plus a max context window size. The info struct
  captures name, version, and provider. Both serialize to JSONB, so adding
  fields later is non-breaking.

- Risk: The Diesel schema macro for the new table must coexist with the
  existing `tasks` schema defined in `src/task/adapters/postgres/schema.rs`
  without conflicts.
  Severity: low
  Likelihood: low
  Mitigation: Each subsystem defines its own `diesel::table!` in its own
  `schema.rs` file, scoped to the subsystem's adapter module. This is the
  established pattern.

- Risk: BDD test step definitions may collide with existing step names.
  Severity: low
  Likelihood: low
  Mitigation: Use backend-specific vocabulary in step definitions (e.g.
  "a backend named" rather than generic "a resource").

## Progress

- [x] Stage A: Domain layer (ids, error, capability/info types, aggregate)
- [x] Stage B: Port layer (repository trait, error types)
- [x] Stage C: In-memory adapter
- [x] Stage D: Service layer (registration service with request types)
- [x] Stage E: Unit tests (domain + service) — 37 tests pass
- [x] Stage F: In-memory integration tests — 5 tests pass
- [x] Stage G: BDD feature tests — 3 scenarios pass
- [x] Stage H: Database migration
- [x] Stage I: Postgres adapter (schema, models, repository)
- [x] Stage J: Postgres integration tests — 5 tests passed
- [x] Stage K: Documentation updates (users-guide, roadmap, design doc decisions)
- [x] Stage L: Final validation (make all, review)

## Surprises & discoveries

- The BDD scenario entry point file cannot share the same name as the step
  definitions directory due to Rust's module resolution rules (`mod X;`
  conflicts when both `X.rs` and `X/mod.rs` exist). Resolved by naming the
  entry point `backend_registration_scenarios.rs` instead of
  `backend_registration_steps.rs`.
- Stages A-I were implemented in a single pass, with only one compilation
  error (forgetting to read `lib.rs` before editing it). The patterns from
  the `task` subsystem transferred cleanly to `agent_backend`.

## Decision log

- Decision: Scope 1.3.1 to registration metadata only, not the full
  `AgentBackend` trait from the design doc.
  Rationale: The design doc's `AgentBackend` trait includes `execute_turn` and
  `translate_tool_schema`, which are roadmap items 1.3.2 and 1.3.3. Item 1.3.1
  focuses on "registration and discovery" — the registry of backend metadata,
  not the runtime dispatch interface. The `AgentBackend` trait will be
  introduced in 1.3.2.
  Date/Author: 2026-02-25

- Decision: Use `BackendStatus` enum with `Active` / `Inactive` states rather
  than hard-deleting backends on deregistration.
  Rationale: Soft-delete via status preserves audit history and allows
  re-activation. The design doc requires audit logging for registration events.
  Hard delete would lose the registration record.
  Date/Author: 2026-02-25

- Decision: Store `capabilities` and `backend_info` as JSONB columns.
  Rationale: Matches the existing pattern (task `origin` is JSONB). Allows
  capability schema evolution without migrations. Both structs derive
  `Serialize`/`Deserialize`.
  Date/Author: 2026-02-25

## Outcomes & retrospective

- All stages completed successfully. Implementation followed the plan with
  minimal deviation.
- The hexagonal architecture pattern from the `task` subsystem replicated
  cleanly to `agent_backend`. No existing tests broke, no public interfaces
  changed, no new crate dependencies were needed.
- Total new files: ~30 (domain, ports, adapters, services, tests,
  migrations, BDD steps). All within the 35-file tolerance.
- Test coverage: 37 unit tests, 5 in-memory integration tests, 3 BDD
  scenarios, 5 Postgres integration tests.
- The BDD entry point naming was the only unexpected issue; documented in
  Surprises above.

## Context and orientation

The Corbusier project lives at `/home/user/project`. It is a single Rust crate
(not a workspace) using edition 2024. The binary is `corbusier`; the library is
`corbusier` (in `src/lib.rs`).

The project follows hexagonal architecture with two existing subsystems:

- `src/message/` — conversation message format, persistence, handoff context
  (roadmap 1.1.x, complete).
- `src/task/` — issue-to-task creation, branch/PR association, state
  transitions (roadmap 1.2.x, complete).

Each subsystem follows an identical layout:

```plaintext
src/<subsystem>/
├── mod.rs              # Module docs, re-exports submodules
├── domain/
│   ├── mod.rs          # Re-exports all domain types
│   ├── ids.rs          # NewType ID wrappers (e.g. TaskId(Uuid))
│   ├── error.rs        # Domain error enum
│   └── <aggregates>.rs # Aggregate roots and value objects
├── ports/
│   ├── mod.rs          # Re-exports port traits
│   └── repository.rs   # Repository trait + error enum
├── adapters/
│   ├── mod.rs
│   ├── memory/         # In-memory adapter for testing
│   └── postgres/       # Production Postgres adapter
│       ├── schema.rs   # diesel::table! macro
│       ├── models.rs   # Row/Insert structs
│       └── repository.rs
├── services/
│   ├── mod.rs
│   └── <service>.rs    # Service with request types
└── tests/              # Unit tests (inside src/)
    └── mod.rs
```

Tests live in two places:

- `src/<subsystem>/tests/` — unit tests for domain logic and service
  orchestration (using `InMemoryRepository` + `mockable::DefaultClock`).
- `tests/` — integration tests:
  - `tests/in_memory/` — multi-step integration scenarios using in-memory
    adapters.
  - `tests/postgres/` — Postgres integration tests using
    `pg-embedded-setup-unpriv` for an ephemeral cluster. Template databases
    are created once with all migrations applied, then cloned per test.
  - `tests/*_steps/` + `tests/features/*.feature` — BDD tests using
    `rstest-bdd`.

Key patterns to replicate:

- **NewType IDs**: `struct BackendId(Uuid)` with `new()`, `from_uuid()`,
  `into_inner()`, `Display`, `Serialize`, `Deserialize`, `Default` (see
  `src/task/domain/ids.rs`).
- **Validated strings**: `struct BackendName(String)` with `new()` that trims
  and validates, `as_str()`, `Display` (see `RepositoryFullName` in
  `src/task/domain/ids.rs`).
- **Immutable aggregates**: Private fields, public getters, `from_persisted()`
  constructor (see `Task` in `src/task/domain/task.rs`).
- **PersistedData struct**: Plain struct with pub fields for reconstructing
  from storage (see `PersistedTaskData`).
- **Repository trait**: `#[async_trait]`, `Send + Sync`, typed error enum with
  `Persistence(Arc<dyn Error + Send + Sync>)` variant (see
  `src/task/ports/repository.rs`).
- **In-memory adapter**: `Arc<RwLock<Inner>>` with `HashMap` storage (see
  `src/task/adapters/memory/task.rs`).
- **Postgres adapter**: `Pool<ConnectionManager<PgConnection>>` type alias,
  `run_blocking` helper for `spawn_blocking`, `to_new_row` / `row_to_*`
  conversions (see `src/task/adapters/postgres/repository.rs`).
- **Service**: Generic over `R: Repository, C: Clock`, takes `Arc<R>` and
  `Arc<C>`, request structs with builder-style setters (see
  `src/task/services/lifecycle.rs`).
- **rstest fixtures**: `#[fixture]` functions, `#[rstest]` + `#[tokio::test]`
  (see `tests/in_memory/task_lifecycle_tests.rs`).
- **BDD tests**: Feature files in `tests/features/`, step modules in
  `tests/*_steps/` with `world.rs`, `given.rs`, `when.rs`, `then.rs`, scenario
  entry points in `tests/*_steps.rs` (see `tests/task_issue_creation_steps.rs`
  and `tests/task_issue_steps/`).
- **Postgres test setup**: `tests/postgres/helpers.rs` includes all migration
  SQL via `include_str!` and applies them in `apply_migrations()`. Each test
  gets a `TemporaryDatabase` cloned from a template.

Build commands: `make check-fmt`, `make lint`, `make test`, `make all` (runs
all three).

## Plan of work

### Stage A: Domain layer

Create `src/agent_backend/domain/` with the following files:

**`src/agent_backend/domain/ids.rs`** — Define `BackendId(Uuid)` following the
`TaskId` pattern exactly. Include `new()`, `from_uuid()`, `into_inner()`,
`Default`, `AsRef<Uuid>`, `Display`, `Serialize`, `Deserialize`, `Clone`,
`Copy`, `PartialEq`, `Eq`, `Hash`.

**`src/agent_backend/domain/name.rs`** — Define `BackendName(String)` as a
validated, non-empty, ASCII-alphanumeric-plus-underscores identifier (like a
slug). Validation: trim, reject empty, reject names with characters outside
`[a-z0-9_]` (lowercase), reject names exceeding 100 chars (matching the
`VARCHAR(100)` column). Include `new()` returning `Result<Self, BackendDomainError>`,
`as_str()`, `Display`, `Serialize`, `Deserialize`, `Clone`, `PartialEq`, `Eq`,
`Hash`.

**`src/agent_backend/domain/status.rs`** — Define `BackendStatus` enum with
variants `Active` and `Inactive`. Include `as_str()`, `Display`, `TryFrom<&str>`
(returning `ParseBackendStatusError`), `Serialize`, `Deserialize`. Follow the
`TaskState` pattern.

**`src/agent_backend/domain/capabilities.rs`** — Define `AgentCapabilities`
struct:

```rust
// src/agent_backend/domain/capabilities.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCapabilities {
    supports_streaming: bool,
    supports_tool_calls: bool,
    supported_content_types: Vec<String>,
    max_context_window: Option<u64>,
}
```

Include a constructor `new(supports_streaming, supports_tool_calls)` with
sensible defaults for the other fields, and builder-style `with_*` methods.
Include public getters for all fields.

**`src/agent_backend/domain/info.rs`** — Define `BackendInfo` struct:

```rust
// src/agent_backend/domain/info.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendInfo {
    display_name: String,
    version: String,
    provider: String,
}
```

Include `new(display_name, version, provider)` constructor and public getters.

**`src/agent_backend/domain/registration.rs`** — Define the
`AgentBackendRegistration` aggregate root:

```rust
// src/agent_backend/domain/registration.rs
pub struct AgentBackendRegistration {
    id: BackendId,
    name: BackendName,
    status: BackendStatus,
    capabilities: AgentCapabilities,
    backend_info: BackendInfo,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

Include:

- `new(name, capabilities, backend_info, clock)` — creates with `Active`
  status and fresh timestamps.
- `from_persisted(data: PersistedBackendData)` — reconstruction from storage.
- Public getters for all fields.
- `deactivate(clock)` — sets status to `Inactive`, touches `updated_at`.
- `activate(clock)` — sets status to `Active`, touches `updated_at`.
- `update_capabilities(capabilities, clock)` — replaces capabilities.

Include `PersistedBackendData` struct with all pub fields for reconstruction.

**`src/agent_backend/domain/error.rs`** — Define:

```rust
// src/agent_backend/domain/error.rs
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum BackendDomainError {
    #[error("backend name must not be empty")]
    EmptyBackendName,
    #[error("backend name '{0}' contains invalid characters (only lowercase alphanumeric and underscores allowed)")]
    InvalidBackendName(String),
    #[error("backend name exceeds 100 character limit: {0}")]
    BackendNameTooLong(String),
    #[error("backend info display name must not be empty")]
    EmptyDisplayName,
    #[error("backend info version must not be empty")]
    EmptyVersion,
    #[error("backend info provider must not be empty")]
    EmptyProvider,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("unknown backend status: {0}")]
pub struct ParseBackendStatusError(pub String);
```

**`src/agent_backend/domain/mod.rs`** — Re-export all domain types.

### Stage B: Port layer

**`src/agent_backend/ports/repository.rs`** — Define the repository trait:

```rust
// src/agent_backend/ports/repository.rs
#[async_trait]
pub trait BackendRegistryRepository: Send + Sync {
    async fn register(&self, registration: &AgentBackendRegistration) -> BackendRegistryResult<()>;
    async fn update(&self, registration: &AgentBackendRegistration) -> BackendRegistryResult<()>;
    async fn find_by_id(&self, id: BackendId) -> BackendRegistryResult<Option<AgentBackendRegistration>>;
    async fn find_by_name(&self, name: &BackendName) -> BackendRegistryResult<Option<AgentBackendRegistration>>;
    async fn list_active(&self) -> BackendRegistryResult<Vec<AgentBackendRegistration>>;
    async fn list_all(&self) -> BackendRegistryResult<Vec<AgentBackendRegistration>>;
}
```

Define error enum:

```rust
#[derive(Debug, Clone, Error)]
pub enum BackendRegistryError {
    #[error("duplicate backend identifier: {0}")]
    DuplicateBackend(BackendId),
    #[error("duplicate backend name: {0}")]
    DuplicateBackendName(BackendName),
    #[error("backend not found: {0}")]
    NotFound(BackendId),
    #[error("persistence error: {0}")]
    Persistence(Arc<dyn std::error::Error + Send + Sync>),
}
```

**`src/agent_backend/ports/mod.rs`** — Re-export port types.

### Stage C: In-memory adapter

**`src/agent_backend/adapters/memory/backend_registry.rs`** — Implement
`InMemoryBackendRegistry` following the `InMemoryTaskRepository` pattern:

- `Arc<RwLock<InMemoryRegistryState>>` with `HashMap<BackendId, AgentBackendRegistration>`
  and a `name_index: HashMap<BackendName, BackendId>`.
- Implement all `BackendRegistryRepository` methods.
- Enforce uniqueness on both `id` and `name` during `register`.

**`src/agent_backend/adapters/memory/mod.rs`** — Re-export.

**`src/agent_backend/adapters/mod.rs`** — Declare `memory` and `postgres`
submodules.

### Stage D: Service layer

**`src/agent_backend/services/registry.rs`** — Define `BackendRegistryService`:

```rust
// src/agent_backend/services/registry.rs
pub struct RegisterBackendRequest {
    name: String,
    display_name: String,
    version: String,
    provider: String,
    supports_streaming: bool,
    supports_tool_calls: bool,
}
```

Include `new()` and builder-style `with_*` methods for optional fields
(content types, max context window).

```rust
pub struct BackendRegistryService<R, C>
where
    R: BackendRegistryRepository,
    C: Clock + Send + Sync,
{
    repository: Arc<R>,
    clock: Arc<C>,
}
```

Methods:

- `register(request) -> Result<AgentBackendRegistration>` — validates,
  constructs domain objects, calls `repository.register()`.
- `find_by_id(id) -> Result<Option<...>>`
- `find_by_name(name) -> Result<Option<...>>` — validates the name string
  first.
- `list_active() -> Result<Vec<...>>`
- `list_all() -> Result<Vec<...>>`
- `deactivate(id) -> Result<AgentBackendRegistration>` — finds, calls
  `deactivate()`, calls `repository.update()`.
- `activate(id) -> Result<AgentBackendRegistration>` — finds, calls
  `activate()`, calls `repository.update()`.

Define `BackendRegistryServiceError`:

```rust
pub enum BackendRegistryServiceError {
    Domain(#[from] BackendDomainError),
    Repository(#[from] BackendRegistryError),
    NotFound(BackendId),
}
```

**`src/agent_backend/services/mod.rs`** — Re-export.

### Stage E: Unit tests

**`src/agent_backend/tests/domain_tests.rs`** — Test:

- `BackendName` validation: empty rejected, invalid chars rejected, too long
  rejected, valid names accepted, case normalization.
- `BackendStatus` round-trip: `as_str()` → `TryFrom<&str>`.
- `BackendInfo` validation: empty display name / version / provider rejected.
- `AgentBackendRegistration` construction: defaults to `Active`, timestamps
  set.
- `deactivate()` and `activate()` state changes and timestamp updates.

**`src/agent_backend/tests/service_tests.rs`** — Test service orchestration
using `InMemoryBackendRegistry`:

- Register and retrieve by ID.
- Register and retrieve by name.
- Duplicate name rejected.
- Deactivate and verify status change.
- List active excludes inactive.
- List all includes inactive.
- Find unknown ID returns None.

**`src/agent_backend/tests/mod.rs`** — Declare submodules.

### Stage F: In-memory integration tests

**`tests/in_memory/backend_registry_tests.rs`** — Integration tests that
exercise the full service → in-memory-adapter path:

- Register two backends, list all, verify count is 2.
- Register, deactivate, list active returns 1 (the success criterion).
- Duplicate name registration fails with `DuplicateBackendName`.

Update **`tests/in_memory.rs`** to declare the new module.

### Stage G: BDD feature tests

**`tests/features/backend_registration.feature`** — Feature file:

```gherkin
Feature: Agent backend registration and discovery

  Scenario: Register two backends and list them
    Given a backend named "claude_code_sdk" from provider "Anthropic"
    And a backend named "codex_cli" from provider "OpenAI"
    When both backends are registered
    Then listing all backends returns 2 entries
    And the backend "claude_code_sdk" can be found by name
    And the backend "codex_cli" can be found by name

  Scenario: Reject duplicate backend name
    Given a backend named "claude_code_sdk" from provider "Anthropic"
    And the backend has already been registered
    When a second backend with name "claude_code_sdk" is registered
    Then registration fails with a duplicate name error

  Scenario: Deactivate a backend and exclude from active listing
    Given a registered backend named "test_backend" from provider "Test"
    When the backend is deactivated
    Then listing active backends does not include "test_backend"
    And listing all backends still includes "test_backend"
```

**`tests/backend_registration_steps/`** — Step definitions following the
`task_issue_steps` pattern: `world.rs`, `given.rs`, `when.rs`, `then.rs`,
`mod.rs`.

**`tests/backend_registration_scenarios.rs`** — Scenario entry points (named
differently from the step directory to avoid Rust module resolution conflicts).

### Stage H: Database migration

**`migrations/2026-02-25-000000_add_backend_registrations_table/up.sql`**:

```sql
-- Add agent backend registration table for roadmap item 1.3.1
-- Follows corbusier-design.md §2.2.3 and §6.2.3.

CREATE TABLE backend_registrations (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    capabilities JSONB NOT NULL,
    backend_info JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT backend_registrations_status_check CHECK (
        status IN ('active', 'inactive')
    )
);

CREATE UNIQUE INDEX idx_backend_registrations_name
    ON backend_registrations (name);
```

**`migrations/2026-02-25-000000_add_backend_registrations_table/down.sql`**:

```sql
DROP TABLE IF EXISTS backend_registrations;
```

### Stage I: Postgres adapter

**`src/agent_backend/adapters/postgres/schema.rs`** — Diesel table definition:

```rust
diesel::table! {
    backend_registrations (id) {
        id -> Uuid,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 50]
        status -> Varchar,
        capabilities -> Jsonb,
        backend_info -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
```

**`src/agent_backend/adapters/postgres/models.rs`** — Define
`BackendRegistrationRow` (Queryable + Selectable + QueryableByName) and
`NewBackendRegistrationRow` (Insertable). Follow the `TaskRow` / `NewTaskRow`
pattern exactly, including explicit `#[diesel(sql_type = ...)]` annotations.

**`src/agent_backend/adapters/postgres/repository.rs`** — Implement
`PostgresBackendRegistry`:

- `BackendPgPool` type alias.
- `run_blocking` helper.
- `register()`: insert with duplicate detection on unique index.
- `update()`: update status, capabilities, backend_info, updated_at.
- `find_by_id()`, `find_by_name()`: single-row lookup.
- `list_active()`: filter by `status = 'active'`.
- `list_all()`: no filter.
- Row-to-domain and domain-to-row conversion functions.

**`src/agent_backend/adapters/postgres/mod.rs`** — Re-export.

### Stage J: Postgres integration tests

**`tests/postgres/backend_registry_tests.rs`** — Postgres integration tests:

- Register and retrieve by ID.
- Register and retrieve by name.
- Duplicate name rejected (unique index violation).
- List active / list all.
- Deactivate and verify exclusion from active listing.

Update **`tests/postgres/helpers.rs`**:

- Add `ADD_BACKEND_REGISTRATIONS_SQL` constant via `include_str!`.
- Add the new migration to `apply_migrations()`.

Update **`tests/postgres.rs`** to declare the new module.

### Stage K: Documentation updates

**`docs/users-guide.md`** — Add a new section "Agent backend registration"
after "Task state transitions", with a `rust,no_run` code example showing
registration of two backends and listing them. Follow the existing style.

**`docs/roadmap.md`** — Mark 1.3.1 and its sub-items as done (`[x]`).

**`docs/corbusier-design.md`** — Insert an implementation decisions block after
§2.2.3 F-003-RQ-001 (after line 929, before F-003-RQ-002 at line 931),
following the established pattern used by roadmap items 1.2.1–1.2.3. The block
will be:

```markdown
###### Implementation decisions (2026-02-25) — roadmap 1.3.1

- Backend registrations are persisted in a dedicated `backend_registrations`
  table with `capabilities` and `backend_info` stored as JSONB columns,
  enabling schema evolution without migrations.
- Backend names (`BackendName`) are validated as lowercase alphanumeric
  identifiers with underscores, limited to 100 characters, and enforced
  unique via a database index.
- Deregistration uses soft-delete via a `BackendStatus` enum (`Active` /
  `Inactive`) rather than hard delete, preserving audit history and
  allowing re-activation.
- The `AgentBackend` runtime trait (with `execute_turn` and
  `translate_tool_schema`) is deferred to roadmap items 1.3.2 and 1.3.3;
  item 1.3.1 introduces only the registration metadata aggregate and
  registry repository.
```

**`src/lib.rs`** — Add `pub mod agent_backend;`.

### Stage L: Final validation

Run `make all`, `make markdownlint`, `make nixie`, and `make fmt` and confirm
all checks pass. Review the diff for:

- No unused imports or dead code.
- No Clippy warnings.
- Formatting compliance (Rust and Markdown).
- Mermaid diagram validity.
- Module-level and public-item doc comments present.

## Concrete steps

All commands run from `/home/user/project`.

1. Create directory structure:

   ```plaintext
   mkdir -p src/agent_backend/{domain,ports,adapters/{memory,postgres},services,tests}
   mkdir -p tests/backend_registration_steps
   mkdir -p migrations/2026-02-25-000000_add_backend_registrations_table
   ```

2. Write domain layer files (Stage A), then run:

   ```bash
   cargo check 2>&1 | head -20
   ```

   Expected: compiles with no errors.

3. Write port layer (Stage B), then `cargo check`.

4. Write in-memory adapter (Stage C), then `cargo check`.

5. Write service layer (Stage D), then `cargo check`.

6. Write unit tests (Stage E), then:

   ```bash
   set -o pipefail; cargo nextest run --lib 2>&1 | tee /tmp/unit-tests.log
   ```

   Expected: all new tests pass.

7. Write in-memory integration tests (Stage F), then:

   ```bash
   set -o pipefail; cargo nextest run --test in_memory 2>&1 | tee /tmp/inmem-tests.log
   ```

   Expected: all tests pass, including new backend registry tests.

8. Write BDD tests (Stage G), then:

   ```bash
   set -o pipefail; cargo nextest run --test backend_registration_scenarios 2>&1 | tee /tmp/bdd-tests.log
   ```

   Expected: all scenarios pass.

9. Write migration (Stage H), then create Postgres adapter
   (Stage I), then:

   ```bash
   set -o pipefail; cargo nextest run --test postgres 2>&1 | tee /tmp/pg-tests.log
   ```

   Expected: all Postgres tests pass, including new backend
   registry tests.

10. Update documentation (Stage K).

11. Final validation:

    ```bash
    set -o pipefail; make all 2>&1 | tee /tmp/make-all.log
    set -o pipefail; make markdownlint 2>&1 | tee /tmp/markdownlint.log
    set -o pipefail; make nixie 2>&1 | tee /tmp/nixie.log
    set -o pipefail; make fmt 2>&1 | tee /tmp/fmt.log
    ```

    Expected: exit code 0 for each command, no warnings, no failures.

## Validation and acceptance

Quality criteria:

- Tests: `make test` passes with zero failures. New tests cover: domain
  validation (happy + unhappy), service orchestration, in-memory integration,
  BDD scenarios, Postgres integration.
- Lint: `make lint` reports zero warnings.
- Format: `make check-fmt` reports zero differences.
- Success criterion from roadmap: "at least two backends can be registered and
  listed via the registry API" — verified by BDD scenario "Register two
  backends and list them" and by the in-memory integration test
  `register_two_backends_and_list`.

Quality method:

```bash
make all
```

## Idempotence and recovery

All test databases are ephemeral (created from template, dropped after test).
All steps are re-runnable. If a migration fails partway, drop the test template
database and re-run; the template creation is idempotent.

If `make all` fails after partial changes, fix the issue and re-run. No manual
cleanup is needed.

## Artifacts and notes

Key reference files for pattern replication:

- `src/task/domain/ids.rs` — NewType ID pattern
- `src/task/domain/task.rs` — Aggregate root pattern
- `src/task/domain/error.rs` — Domain error pattern
- `src/task/ports/repository.rs` — Repository trait pattern
- `src/task/adapters/memory/task.rs` — In-memory adapter pattern
- `src/task/adapters/postgres/repository.rs` — Postgres adapter pattern
- `src/task/adapters/postgres/schema.rs` — Diesel schema pattern
- `src/task/adapters/postgres/models.rs` — Diesel model pattern
- `src/task/services/lifecycle.rs` — Service + request pattern
- `tests/in_memory/task_lifecycle_tests.rs` — rstest integration test pattern
- `tests/task_issue_creation_steps.rs` — BDD entry point pattern
- `tests/task_issue_steps/world.rs` — BDD world pattern
- `tests/features/task_issue_creation.feature` — Feature file pattern
- `tests/postgres/helpers.rs` — Postgres test helper pattern
- `tests/postgres/task_lifecycle_tests.rs` — Postgres test pattern
- `migrations/2026-02-09-000000_add_tasks_table/up.sql` — Migration pattern

## Interfaces and dependencies

No new external crate dependencies. All required crates are already in
`Cargo.toml`: `uuid`, `chrono`, `serde`, `serde_json`, `thiserror`,
`async-trait`, `diesel`, `tokio`, `mockable`, `rstest`, `rstest-bdd`.

### New public types (at completion)

In `src/agent_backend/domain/`:

```rust
pub struct BackendId(Uuid);           // ids.rs
pub struct BackendName(String);       // name.rs
pub enum BackendStatus { Active, Inactive }  // status.rs
pub struct AgentCapabilities { .. }   // capabilities.rs
pub struct BackendInfo { .. }         // info.rs
pub struct AgentBackendRegistration { .. }    // registration.rs
pub struct PersistedBackendData { .. }       // registration.rs
pub enum BackendDomainError { .. }    // error.rs
pub struct ParseBackendStatusError(pub String);  // error.rs
```

In `src/agent_backend/ports/`:

```rust
pub trait BackendRegistryRepository: Send + Sync { .. }  // repository.rs
pub enum BackendRegistryError { .. }                     // repository.rs
pub type BackendRegistryResult<T> = Result<T, BackendRegistryError>;
```

In `src/agent_backend/adapters/`:

```rust
pub struct InMemoryBackendRegistry { .. }  // memory/backend_registry.rs
pub struct PostgresBackendRegistry { .. }  // postgres/repository.rs
pub type BackendPgPool = Pool<ConnectionManager<PgConnection>>;
```

In `src/agent_backend/services/`:

```rust
pub struct RegisterBackendRequest { .. }      // registry.rs
pub struct BackendRegistryService<R, C> { .. } // registry.rs
pub enum BackendRegistryServiceError { .. }   // registry.rs
```

### New database table

```sql
CREATE TABLE backend_registrations (
    id UUID PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    capabilities JSONB NOT NULL,
    backend_info JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT backend_registrations_status_check CHECK (
        status IN ('active', 'inactive')
    )
);

CREATE UNIQUE INDEX idx_backend_registrations_name
    ON backend_registrations (name);
```
