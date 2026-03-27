# Establish tenant primitives and request context plumbing (Roadmap 2.5.1)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: COMPLETE

## Purpose / big picture

Corbusier is an AI agent orchestration platform that manages conversations,
tasks, agent backends, and tool registries. Today, every repository and service
method operates without knowing which tenant owns the data. There is a small
`AuditContext` struct in the message adapter layer that carries four optional
bare `Uuid` fields (`correlation_id`, `causation_id`, `user_id`, `session_id`),
but it is not used in any port trait signatures and only surfaces in one
Postgres-specific `store_with_audit()` method.

This plan introduces three things:

1. A new `tenant` bounded context (`src/tenant/`) containing domain primitives
   `TenantId`, `TenantSlug`, and `Tenant`.
2. A new cross-cutting `context` module (`src/context/`) containing
   `RequestContext` and newtype identifiers `CorrelationId`, `CausationId`,
   `UserId`, and `SessionId`.
3. Updated port trait signatures across every repository and handoff port so
   that tenant-owned operations require a `&RequestContext` parameter.

After this change:

- All repository and service methods that operate on tenant-owned data accept a
  `&RequestContext` carrying `tenant_id`, `correlation_id`, `causation_id`,
  `user_id`, and `session_id`.
- The existing `AuditContext` is replaced by a `From<&RequestContext>`
  conversion that the Postgres adapters use internally.
- A `Tenant` domain aggregate with `TenantId`, `TenantSlug`, display name,
  status, and timestamps is available for future persistence (the database
  migration is deferred to 2.5.2).
- `make all` passes with all existing and new tests green.

This is step 1 of 6 in the multi-tenancy delivery sequence. Later steps
(2.5.2-2.5.4) will deliver schema migrations, adapter-level tenant filtering,
Row-Level Security (RLS) policies, and two-tenant isolation tests. This step
focuses exclusively on domain primitives and plumbing signatures.

## Constraints

- Hexagonal architecture must be preserved: domain types must have zero
  infrastructure imports; ports define trait contracts; adapters implement them.
- All existing tests must continue to pass. No regressions.
- Rust edition 2024. All Clippy warnings denied (`-D warnings`).
- No `unsafe` code.
- Module-level doc comments (`//!`) required on every module.
- Rustdoc comments (`///`) required on every public item, including `# Errors`
  sections on fallible methods.
- Commit gating: `make check-fmt && make lint && make test` must pass before
  each commit.
- No database schema migration in this plan; the `tenants` table and
  `tenant_id` columns are deferred to roadmap 2.5.2.
- The `SlashCommandRegistry` port is not tenant-scoped (it loads static
  definitions, not tenant-owned data) and must not be changed.
- The `worker` module is a shell-escape utility, not a bounded context, and
  must not be changed.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 50 files (net) or
  2500 lines of new/changed code, stop and escalate.
- Dependencies: if a new external crate dependency is required beyond what is
  already in `Cargo.toml`, stop and escalate.
- Iterations: if tests still fail after 5 fix-and-rerun cycles in any single
  milestone, stop and escalate.
- Ambiguity: if multiple valid interpretations exist and the choice materially
  affects downstream roadmap items (2.5.2-2.5.4), stop and present options.

## Risks

- Risk: Adding `&RequestContext` to every port method is a wide cross-cutting
  change that touches every adapter implementation and every test call-site.
  Severity: medium Likelihood: high Mitigation: Stage the work carefully —
  update one port trait at a time, fix all adapters and tests for that port,
  then move to the next. Keep commits atomic per port. This also ensures the
  build stays green between stages.

- Risk: Existing Postgres adapter's `store_with_audit()` method on
  `PostgresMessageRepository` takes `&AuditContext` directly. Replacing this
  may break the public interface contract of the adapter. Severity: low
  Likelihood: high Mitigation: `store_with_audit()` is an adapter-only method
  (not on the port trait). Change its signature to accept `&RequestContext` and
  convert internally. Keep the `AuditContext` struct as an internal adapter
  helper with `From<&RequestContext>`.

- Risk: The `TenantSlug` validation rules are not fully specified in the design
  doc beyond "suitable for URLs and config keys." Severity: low Likelihood:
  high Mitigation: Use the same validation pattern as `BackendName` (lowercase
  alphanumeric plus hyphens, max 63 characters to match Domain Name System
  (DNS) label rules). Hyphens (not underscores) are more URL-conventional.
  Document the choice in the decision log.

- Risk: Behaviour-Driven Development (BDD) test step definitions
  may collide with existing step names from other subsystems. Severity: low
  Likelihood: low Mitigation: Use tenant-specific vocabulary in step
  definitions.

## Progress

- [x] Stage A: Cross-cutting context module (`src/context/`)
- [x] Stage B: Tenant domain primitives (`src/tenant/`)
- [x] Stage C: Update port traits to accept `&RequestContext`
- [x] Stage D: Update adapter implementations (in-memory and Postgres)
- [x] Stage E: Update service layers to forward `RequestContext`
- [x] Stage F: Unit tests for context and tenant domain
- [x] Stage G: Update existing integration tests with `RequestContext`
- [x] Stage H: BDD feature tests for tenant domain
- [x] Stage I: Documentation updates (users-guide, roadmap, design doc)
- [x] Stage J: Final validation (`make all`)

## Surprises & discoveries

- The project enforces `clippy::allow_attributes` and
  `clippy::allow_attributes_without_reason`, requiring
  `#[expect(clippy::..., reason = "...")]` instead of `#[allow(clippy::...)]`.
  This affected several locations where `too_many_arguments` was suppressed.
- Adding `&RequestContext` to rstest fixture-injected test functions pushed many
  over the 4-argument Clippy limit. These were handled with `#[expect]`
  annotations since rstest requires individual parameters.
- The existing `audit_tests.rs` pattern of testing with `ExpectedAuditContext`
  (including an "empty context" case with all `None` fields) was incompatible
  with `RequestContext` which always has required fields. Redesigned to
  parameterize on the single optional field (`causation_id`).
- `TenantStatus::Display` outputs lowercase (`"active"`, `"suspended"`),
  matching the `BackendStatus` convention. BDD feature files needed lowercase
  values.

## Decision log

- Decision: Place `RequestContext` in a new top-level `src/context/` module
  rather than under `src/tenant/`. Rationale: `RequestContext` is cross-cutting
  — it carries correlation, causation, user, session, and tenant identifiers
  and is used by every bounded context. Placing it under `src/tenant/` would
  create a dependency from every other module on the tenant module. A separate
  `src/context/` module keeps dependencies clean: `context` depends on `tenant`
  (for `TenantId`), and every other module depends on `context`. Date:
  2026-03-03

- Decision: Keep `AuditContext` as an internal adapter type with
  `From<&RequestContext>`, rather than deleting it entirely. Rationale:
  `AuditContext` encapsulates the specific fields that map to PostgreSQL
  session variables for audit triggers. It serves as an adapter-internal
  concern. Keeping it avoids coupling the Postgres adapter's
  `set_audit_context()` SQL helper directly to the domain-level
  `RequestContext`. The conversion is mechanical and tested. Date: 2026-03-03

- Decision: Use lowercase alphanumeric plus hyphens for `TenantSlug`, max 63
  characters (DNS label convention), rather than underscores like
  `BackendName`. Rationale: Slugs are URL-facing identifiers. Hyphens are the
  standard word separator in URLs and DNS labels. The 63-character limit
  matches DNS label constraints and is sufficient for tenant identifiers. Date:
  2026-03-03

- Decision: Add `&RequestContext` to all port methods (reads and writes), not
  just writes. Rationale: The design doc states "every state mutation and
  lookup executes within a tenant context." Future steps (2.5.3) will add
  tenant filtering to reads. Adding the parameter now avoids a second
  cross-cutting signature change later. The read path can initially ignore the
  tenant_id, but having it in the signature is the contract that callers must
  provide context. Date: 2026-03-03

- Decision: `SlashCommandRegistry` is excluded from `RequestContext` plumbing.
  Rationale: It loads static command definitions, not tenant-owned data. Date:
  2026-03-03

## Outcomes & retrospective

Completed 2026-03-04. All stages delivered successfully.

- `make check-fmt`, `make lint`, and `make test` all pass.
- 659 tests run (659 passed, 1 skipped).
- New modules: `src/context/` (5 ID newtypes, `RequestContext`), `src/tenant/`
  (`TenantId`, `TenantSlug`, `TenantStatus`, `Tenant`, `TenantDomainError`).
- 6 port traits updated with `&RequestContext` on all methods.
- All in-memory and Postgres adapters updated.
- All service layers updated.
- `AuditContext` retained as adapter-internal type with `From<&RequestContext>`.
- 4 new BDD scenarios for tenant identity.
- User guide, design doc, and roadmap updated.
- No new external dependencies added.
- The `#[expect(clippy::..., reason = "...")]` pattern was required in several
  places where `too_many_arguments` needed suppression on rstest-injected test
  functions — this was the primary source of iteration during Stage G.

## Context and orientation

The codebase lives in a single Rust crate (`corbusier`) with seven top-level
modules declared in `src/lib.rs`:

```plaintext
src/lib.rs          -> pub mod agent_backend, context, message, task, tenant, tool_registry, worker
src/context/        -> ids, request_context, tests
src/tenant/         -> domain, mod
src/agent_backend/  -> domain, ports, adapters (memory + postgres), services
src/message/        -> domain, ports, adapters (memory + postgres), services, validation, tests
src/task/           -> domain, ports, adapters (memory + postgres), services
src/tool_registry/  -> domain, ports, adapters (memory + postgres), services
src/worker.rs       -> shell_escape utility (not a bounded context)
```

Key files for this plan:

- `src/message/adapters/audit_context.rs` — existing `AuditContext` struct
  (4 `Option<Uuid>` fields, builder pattern). Lives in adapter layer.
- `src/message/adapters/postgres/sql_helpers.rs` — `set_audit_context()`
  function that sets PostgreSQL session variables from `AuditContext`.
- `src/message/adapters/postgres/mod.rs` — `store_with_audit()` method.
- Port traits to update:
  - `src/message/ports/repository.rs` — `MessageRepository` (5 methods)
  - `src/message/ports/agent_session.rs` — `AgentSessionRepository` (5 methods)
  - `src/message/ports/context_snapshot.rs` — `ContextSnapshotPort` (4 methods)
  - `src/message/ports/handoff.rs` — `AgentHandoffPort` (5 methods)
  - `src/agent_backend/ports/repository.rs` — `BackendRegistryRepository`
    (6 methods)
  - `src/task/ports/repository.rs` — `TaskRepository` (6 methods)
- Adapter implementations (in-memory and Postgres) for each of the above ports.
- Service layers that call the port methods.
- Test files under `tests/in_memory/`, `tests/postgres/`, and
  `tests/features/` plus step definitions.

Existing ID newtype pattern (`src/agent_backend/domain/ids.rs`): UUID wrapping
struct with `new()`, `from_uuid()`, `into_inner()`, `Default`, `AsRef<Uuid>`,
`Display`. Derives:
`Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize` with
`#[serde(transparent)]`.

Existing validated string pattern (`src/agent_backend/domain/name.rs`):
`BackendName(String)` with `new(impl Into<String>) -> Result<Self, Error>`,
trimming, normalization, character validation, length check. Derives:
`Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize` with
`#[serde(transparent)]`. Implements `AsRef<str>` and `Display`.

Quality gates (from `AGENTS.md`): `make check-fmt`, `make lint`, `make test`
before every commit. Makefile uses
`cargo nextest run --all-targets --all-features` for tests.

## Plan of work

### Stage A: Cross-cutting context module (`src/context/`)

Create `src/context/` with newtype ID wrappers and `RequestContext`.

**`src/context/mod.rs`** — module root exporting all public types.

**`src/context/ids.rs`** — four UUID newtypes following the established pattern:

- `CorrelationId` — links operations within a single user request.
- `CausationId` — points to the domain event that caused this operation.
- `UserId` — the actor performing the operation.
- `SessionId` — the current user session.

Each follows the exact same pattern as `BackendId`: `new()`, `from_uuid()`,
`into_inner()`, `Default`, `AsRef<Uuid>`, `Display`, plus the standard derive
set.

**`src/context/request_context.rs`** — the `RequestContext` struct:

```rust
// src/context/request_context.rs
use crate::context::ids::{CausationId, CorrelationId, SessionId, UserId};
use crate::tenant::TenantId;

/// Cross-cutting request context for tenant-scoped operations.
///
/// Every repository and service method operating on tenant-owned data
/// requires a `RequestContext`. It carries the tenant identity, distributed
/// tracing identifiers, and the authenticated principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    tenant_id: TenantId,
    correlation_id: CorrelationId,
    causation_id: Option<CausationId>,
    user_id: UserId,
    session_id: SessionId,
}
```

Private fields with getter methods and a builder or constructor. A
`RequestContext::new()` taking all required fields, plus `with_causation_id()`
for the optional field.

**`src/context/tests.rs`** — unit tests for `RequestContext` construction,
accessors, equality, and clone.

Register the module in `src/lib.rs`: `pub mod context;`

**Validation**: `cargo check` compiles with no errors.

### Stage B: Tenant domain primitives (`src/tenant/`)

Create `src/tenant/` following the established subsystem layout.

**`src/tenant/mod.rs`** — module root re-exporting domain types.

**`src/tenant/domain/mod.rs`** — domain module root.

**`src/tenant/domain/ids.rs`** — `TenantId` UUID newtype (same pattern as
`BackendId`).

**`src/tenant/domain/slug.rs`** — `TenantSlug` validated string type:

```rust
// src/tenant/domain/slug.rs
/// Validated tenant slug suitable for URLs and configuration keys.
///
/// Slugs are lowercased, 1-63 characters, containing only `[a-z0-9-]`,
/// must start and end with an alphanumeric character, and must not contain
/// consecutive hyphens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TenantSlug(String);
```

Validation rules: trim, lowercase, reject empty, reject characters outside
`[a-z0-9-]`, reject leading/trailing hyphens, reject consecutive hyphens, max
63 chars. Returns `Result<Self, TenantDomainError>`.

**`src/tenant/domain/status.rs`** — `TenantStatus` enum:

```rust
// src/tenant/domain/status.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TenantStatus {
    Active,
    Suspended,
}
```

With `Display` and a `from_str` or `TryFrom<&str>` for persistence
round-tripping. Following the `BackendStatus` pattern.

**`src/tenant/domain/tenant.rs`** — `Tenant` aggregate:

```rust
// src/tenant/domain/tenant.rs
pub struct Tenant {
    id: TenantId,
    slug: TenantSlug,
    display_name: String,
    owner_user_id: UserId,
    status: TenantStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

Constructor `Tenant::new(slug, display_name, owner_user_id, clock)` plus a
`from_persisted()` for rehydration. Getter methods for all fields. The
`owner_user_id` field models the "one owning user per tenant" requirement while
keeping user and tenant identities separate (the `UserId` is a context-layer
type, not a tenant-layer type, preserving the distinction).

**`src/tenant/domain/error.rs`** — `TenantDomainError` enum covering slug
validation failures, empty display name, and status parsing errors.

Register the module in `src/lib.rs`: `pub mod tenant;`

**Validation**: `cargo check` compiles. Unit tests for slug validation (happy
path, empty, invalid chars, too long, leading/trailing hyphens, consecutive
hyphens), tenant construction, and status round-tripping.

### Stage C: Update port traits to accept `&RequestContext`

Add `use crate::context::RequestContext;` to each port module and add
`ctx: &RequestContext` as the first parameter to every method on the following
traits:

1. `MessageRepository` in `src/message/ports/repository.rs` (5 methods)
2. `AgentSessionRepository` in `src/message/ports/agent_session.rs` (5 methods)
3. `ContextSnapshotPort` in `src/message/ports/context_snapshot.rs` (4 methods)
4. `AgentHandoffPort` in `src/message/ports/handoff.rs` (5 methods)
5. `BackendRegistryRepository` in `src/agent_backend/ports/repository.rs`
   (6 methods)
6. `TaskRepository` in `src/task/ports/repository.rs` (6 methods)

Example signature change for `MessageRepository::store`:

```rust
// Before:
async fn store(&self, message: &Message) -> RepositoryResult<()>;

// After:
async fn store(&self, ctx: &RequestContext, message: &Message) -> RepositoryResult<()>;
```

This is a mechanical transformation. Every method gains `ctx: &RequestContext`
as its first parameter after `&self`.

**Note**: `SlashCommandRegistry` in `src/message/ports/slash_command.rs` is
excluded — it loads static command definitions, not tenant-owned data.

**Validation**: `cargo check` will fail (adapters and call-sites not yet
updated). This is expected; proceed to Stage D.

### Stage D: Update adapter implementations

Update every adapter that implements a changed port trait to accept the new
`ctx: &RequestContext` parameter.

**In-memory adapters** (6 files):

- `src/message/adapters/memory/message.rs` — `InMemoryMessageRepository`
- `src/message/adapters/memory/agent_session.rs` — in-memory session impl
- `src/message/adapters/memory/context_snapshot.rs` — in-memory snapshot impl
- `src/message/adapters/memory/handoff.rs` — in-memory handoff impl
- `src/agent_backend/adapters/memory/mod.rs` — `InMemoryBackendRegistry`
- `src/task/adapters/memory/mod.rs` — in-memory task repo

For in-memory adapters, add the `ctx` parameter and prefix it with `_ctx`
(unused for now — tenant filtering comes in 2.5.3).

**Postgres adapters** (3+ files):

- `src/message/adapters/postgres/mod.rs` — `PostgresMessageRepository`
- `src/agent_backend/adapters/postgres/mod.rs` — `PostgresBackendRegistry`
- `src/task/adapters/postgres/mod.rs` — `PostgresTaskRepository`

For Postgres adapters, add the `ctx` parameter. In the message adapter's
`store()` (and `store_with_audit()`), convert `RequestContext` to
`AuditContext` using the new `From<&RequestContext>` impl and call the existing
`set_audit_context()` SQL helper. The `store_with_audit()` method signature
changes from taking `&AuditContext` to `&RequestContext`.

**`AuditContext` conversion** — add to `src/message/adapters/audit_context.rs`:

```rust
impl From<&RequestContext> for AuditContext {
    fn from(ctx: &RequestContext) -> Self {
        Self {
            correlation_id: Some(ctx.correlation_id().into_inner()),
            causation_id: ctx.causation_id().map(CausationId::into_inner),
            user_id: Some(ctx.user_id().into_inner()),
            session_id: Some(ctx.session_id().into_inner()),
        }
    }
}
```

**Validation**: `cargo check` compiles (may still have test failures from
call-site mismatches — addressed in Stage E and G).

### Stage E: Update service layers to forward `RequestContext`

Update every service method to accept `ctx: &RequestContext` and forward it to
repository calls.

- `src/agent_backend/services/registry.rs` — `BackendRegistryService` methods:
  `register`, `find_by_id`, `find_by_name`, `list_active`, `list_all`,
  `deactivate`, `activate`, plus private helpers `update_status` and
  `find_by_id_or_error`.
- `src/message/services/handoff.rs` — handoff service methods.
- Any other service files under `src/message/services/` and
  `src/task/services/`.

Example:

```rust
// Before:
pub async fn register(&self, request: RegisterBackendRequest)
    -> BackendRegistryServiceResult<AgentBackendRegistration> {
    // ...
    self.repository.register(&registration).await?;

// After:
pub async fn register(&self, ctx: &RequestContext, request: RegisterBackendRequest)
    -> BackendRegistryServiceResult<AgentBackendRegistration> {
    // ...
    self.repository.register(ctx, &registration).await?;
```

**Validation**: `cargo check` compiles. All production code is updated.

### Stage F: Unit tests for context and tenant domain

Add comprehensive unit tests:

**`src/context/tests.rs`** (or `src/context/mod.rs` `#[cfg(test)]` block):

- `RequestContext::new()` construction with all required fields.
- `with_causation_id()` builder method.
- Getter methods return correct values.
- Clone and equality semantics.
- Each ID newtype: `new()` generates unique values, `from_uuid()` round-trips,
  `Display` format, `Default`.

**`src/tenant/domain/tests.rs`** (or per-file test modules):

- `TenantId`: same tests as other ID types.
- `TenantSlug`: valid slugs, empty rejection, invalid characters, too long,
  leading hyphen, trailing hyphen, consecutive hyphens, trimming and
  lowercasing, `as_str()` and `Display`.
- `TenantStatus`: `Display` and parse round-trip, unknown status string
  rejection.
- `Tenant`: construction, getter methods, `owner_user_id` distinct from
  `TenantId`, created_at/updated_at set by clock.

**Validation**: `cargo test --lib` passes for new test modules.

### Stage G: Update existing integration and BDD tests

Update all test call-sites to provide a `RequestContext` where port methods are
called.

Create a test helper fixture:

```rust
// In test helpers (could be tests/test_context.rs or a shared module)
use corbusier::context::RequestContext;
use corbusier::context::ids::{CorrelationId, SessionId, UserId};
use corbusier::tenant::TenantId;

/// Creates a default `RequestContext` for tests.
fn test_request_context() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}
```

Update files:

- `tests/in_memory/backend_registry_tests.rs`
- `tests/in_memory/conversation_flow_tests.rs`
- `tests/in_memory/slash_command_tests.rs`
- `tests/in_memory/handoff_tests/` (all files)
- `tests/postgres/backend_registry_tests.rs`
- `tests/postgres/audit_tests.rs`
- `tests/postgres/handoff_tests.rs`
- `tests/postgres/helpers.rs` (update `insert_conversation`, helper functions)
- BDD world structs and step definitions in `tests/*_steps/`
- BDD scenario entry points

Each test that calls a repository or service method must now pass a
`&RequestContext`. Where existing tests use `AuditContext` directly (e.g.,
`store_with_audit()`), convert to use `RequestContext`.

**Validation**: `make test` passes with all existing tests green.

### Stage H: BDD feature tests for tenant domain

Create BDD tests for the tenant domain lifecycle:

**`tests/features/tenant_identity.feature`**:

```gherkin
Feature: Tenant identity and domain primitives

  Scenario: Create a tenant with valid slug and display name
    Given a tenant slug "acme-corp" with display name "Acme Corporation"
    When the tenant is created
    Then the tenant has a unique identifier
    And the tenant slug is "acme-corp"
    And the tenant status is "Active"

  Scenario: Reject tenant creation with invalid slug
    Given a tenant slug "INVALID SLUG!!" with display name "Bad Tenant"
    When the tenant creation is attempted
    Then tenant creation fails with a slug validation error
```

**`tests/tenant_identity_steps/`** — step definitions directory with `mod.rs`,
`world.rs`, `given.rs`, `when.rs`, `then.rs`.

**`tests/tenant_identity_scenarios.rs`** — BDD scenario entry point (note: name
differs from directory per established convention).

**Validation**: `make test` passes including new BDD scenarios.

### Stage I: Documentation updates

- `docs/users-guide.md` — add a section on tenant context explaining that all
  operations now require a `RequestContext` and what its fields mean.
- `docs/roadmap.md` — mark task 2.5.1 as done (change `- [ ]` to `- [x]`).
- `docs/corbusier-design.md` — add implementation decisions under §2.2.5 for
  the choices made (slug validation rules, `RequestContext` module location,
  `AuditContext` migration approach).

**Validation**: `make markdownlint`, `make fmt`, and `make nixie` all pass;
manual review.

### Stage J: Final validation

Run the full quality gate:

```bash
set -o pipefail; make check-fmt 2>&1 | tee /tmp/1-5-1-check-fmt.log
set -o pipefail; make markdownlint 2>&1 | tee /tmp/1-5-1-markdownlint.log
set -o pipefail; make lint 2>&1 | tee /tmp/1-5-1-lint.log
set -o pipefail; make test 2>&1 | tee /tmp/1-5-1-test.log
set -o pipefail; make nixie 2>&1 | tee /tmp/1-5-1-nixie.log
set -o pipefail; make fmt 2>&1 | tee /tmp/1-5-1-fmt.log
```

All commands must exit 0.

## Concrete steps

All commands run from the repository root `/home/user/project`.

Stage A-B (new modules):

```bash
# After writing src/context/ and src/tenant/ files:
cargo check 2>&1 | head -50
```

Expected: `Finished` with no errors.

Stage C-E (signature changes):

```bash
# After updating all ports, adapters, and services:
cargo check 2>&1 | head -50
```

Expected: `Finished` with no errors.

Stage F-H (tests):

```bash
set -o pipefail; make test 2>&1 | tee /tmp/1-5-1-test.log
```

Expected: all tests pass, including new unit tests, updated integration tests,
and new BDD scenarios.

Stage J (final):

```bash
set -o pipefail; make check-fmt 2>&1 | tee /tmp/1-5-1-check-fmt.log
set -o pipefail; make markdownlint 2>&1 | tee /tmp/1-5-1-markdownlint.log
set -o pipefail; make lint 2>&1 | tee /tmp/1-5-1-lint.log
set -o pipefail; make test 2>&1 | tee /tmp/1-5-1-test.log
set -o pipefail; make nixie 2>&1 | tee /tmp/1-5-1-nixie.log
set -o pipefail; make fmt 2>&1 | tee /tmp/1-5-1-fmt.log
```

Expected: all six commands must exit 0.

## Validation and acceptance

Quality criteria:

- Tests: `make test` passes. New tests include:
  - Unit tests for `CorrelationId`, `CausationId`, `UserId`, `SessionId`
    (construction, round-trip, display, default).
  - Unit tests for `RequestContext` (construction, accessors, clone, equality).
  - Unit tests for `TenantId` (construction, round-trip, display, default).
  - Unit tests for `TenantSlug` (valid, empty, invalid chars, too long,
    leading/trailing hyphens, consecutive hyphens).
  - Unit tests for `TenantStatus` (display, parse round-trip).
  - Unit tests for `Tenant` (construction, getters, owner_user_id).
  - Unit tests for `AuditContext::from(&RequestContext)` conversion.
  - BDD scenarios for tenant creation (happy and unhappy paths).
  - All existing integration tests pass with `RequestContext` plumbing.
- Lint: `make lint` passes (Clippy `-D warnings` and `cargo doc --no-deps`).
- Format: `make check-fmt` passes.

Quality method:

```bash
make all   # runs check-fmt, lint, test
```

## Idempotence and recovery

All stages create or modify source files only. No database state is changed
(migrations are deferred to 2.5.2). Re-running any stage overwrites files
idempotently. If a stage fails partway, fix the issue and re-run `make all`
from the repository root.

## Artifacts and notes

Key type signatures after completion:

```rust
// src/context/request_context.rs
pub struct RequestContext {
    tenant_id: TenantId,
    correlation_id: CorrelationId,
    causation_id: Option<CausationId>,
    user_id: UserId,
    session_id: SessionId,
}

// src/tenant/domain/tenant.rs
pub struct Tenant {
    id: TenantId,
    slug: TenantSlug,
    display_name: String,
    owner_user_id: UserId,
    status: TenantStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

Port trait signature pattern after completion:

```rust
// Example: src/message/ports/repository.rs
#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn store(&self, ctx: &RequestContext, message: &Message) -> RepositoryResult<()>;
    async fn find_by_id(&self, ctx: &RequestContext, id: MessageId) -> RepositoryResult<Option<Message>>;
    // ... all methods gain ctx: &RequestContext as first parameter
}
```

## Interfaces and dependencies

No new external crate dependencies. All types use existing crates: `uuid`,
`serde`, `chrono`, `thiserror`, `async-trait`.

New public modules and types:

```plaintext
crate::context                          (new module)
crate::context::RequestContext          (struct)
crate::context::CorrelationId           (newtype)
crate::context::CausationId             (newtype)
crate::context::UserId                  (newtype)
crate::context::SessionId               (newtype)

crate::tenant                           (new module)
crate::tenant::TenantId                 (newtype)
crate::tenant::TenantSlug               (validated string)
crate::tenant::TenantStatus             (enum)
crate::tenant::Tenant                   (aggregate)
crate::tenant::TenantDomainError        (error enum)
crate::tenant::ParseTenantStatusError   (error struct)
```

Changed port traits (all gain `ctx: &RequestContext` on every method):

```plaintext
crate::message::ports::repository::MessageRepository
crate::message::ports::agent_session::AgentSessionRepository
crate::message::ports::context_snapshot::ContextSnapshotPort
crate::message::ports::handoff::AgentHandoffPort
crate::agent_backend::ports::repository::BackendRegistryRepository
crate::task::ports::repository::TaskRepository
```

Unchanged:

```plaintext
crate::message::ports::slash_command::SlashCommandRegistry   (static definitions)
crate::message::ports::validator::*                          (validation, not persistence)
```
