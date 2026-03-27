# Deliver tenant-aware schema and constraints (roadmap 1.5.2)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DONE

## Purpose / big picture

Roadmap item 1.5.1 introduced `TenantId`, `TenantSlug`, `Tenant`, and the
cross-cutting `RequestContext`, so every tenant-owned service and repository
call now receives tenant identity explicitly. The database schema still lags
behind that contract. Core tables such as `tasks`, `backend_registrations`,
`conversations`, `messages`, `agent_sessions`, `handoffs`, and
`context_snapshots` do not yet persist `tenant_id`, task issue-origin
uniqueness is still global, backend registration names are still globally
unique in PostgreSQL, and child tables do not yet have composite foreign keys
that prevent cross-tenant parent/child links.

After this change, Corbusier will store tenant identity directly in the schema,
allow the same issue reference and the same backend name to exist in different
tenants without collision, and reject cross-tenant parent/child relationships
at the database layer. Observable success is:

1. New `rstest` PostgreSQL integration tests prove that:
   - tenant A and tenant B can both create a task from the same external issue
     reference,
   - tenant A and tenant B can both register the same backend name, and
   - composite foreign keys reject tenant-mismatched child rows.
2. Behaviour tests written with `rstest-bdd` prove the service-level workflow
   for same-reference and same-name reuse across two tenants where that
   behaviour is user-visible.
3. `make check-fmt`, `make lint`, `make test TEST_FLAGS='--profile long
   --all-targets --all-features'`, `make fmt`, `make markdownlint`, and `make
   nixie` all pass.
4. `docs/corbusier-design.md`, `docs/users-guide.md`, and `docs/roadmap.md`
   are updated, and roadmap item 1.5.2 is marked done only after every quality
   gate passes.

This milestone is intentionally narrower than 1.5.3. It delivers schema,
uniqueness, and composite foreign keys. It does not deliver full adapter-wide
query scoping or PostgreSQL Row-Level Security (RLS); those remain the work of
1.5.3.

## Constraints

- Preserve hexagonal boundaries. Domain types remain unaware of Diesel,
  PostgreSQL, or migration details. Port trait signatures remain unchanged from
  1.5.1. All tenant-aware persistence logic belongs in adapters.
- Scope the work to roadmap 1.5.2. Do not enable RLS policies, do not add
  `set_config('app.tenant_id', ...)` plumbing beyond existing helpers, and do
  not broaden the work into the full adapter isolation milestone of 1.5.3.
- Keep existing `RequestContext`-based public service and repository APIs
  stable. If a port trait or domain constructor must change, stop and escalate.
- Use additive PostgreSQL migrations with reversible `down.sql` files.
  Existing data must remain migratable via a deterministic backfill path.
- Use `rstest` for unit and integration fixtures, and `rstest-bdd` for
  behaviour tests where a user-observable workflow exists.
- Use the existing `pg_embedded_setup_unpriv`-backed PostgreSQL harness
  through `tests/postgres/helpers.rs`; do not add a second Postgres test
  bootstrap path.
- Keep files under the repository's 400-line limit by splitting modules or
  test files early instead of allowing one large migration helper or test file
  to accrete.
- Maintain en-GB-oxendict spelling in comments and documentation.
- No new external dependencies are authorized for this milestone.
- Update `docs/corbusier-design.md` with implementation decisions taken during
  execution, update `docs/users-guide.md` with any user-visible tenant-aware
  behaviour, and mark `docs/roadmap.md` done only after the feature is fully
  implemented and validated.

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 35 files or more than
  2,500 net lines, stop and document a narrower split before proceeding.
- Interfaces: if a port trait outside adapter internals must change, stop and
  escalate.
- Dependencies: if the work appears to require a new crate, Diesel macro
  plugin, or third-party migration helper, stop and escalate.
- Ambiguity: if the boundary between 1.5.2 and 1.5.3 becomes unclear enough
  that the implementation would either leave same-tenant lookups incorrect or
  effectively deliver most of 1.5.3, stop and present options.
- Data migration: if a safe default-tenant backfill cannot be expressed
  cleanly in `up.sql` and `down.sql`, stop and escalate.
- Iterations: if any failing quality gate remains unresolved after 4 focused
  fix-and-rerun cycles, stop and document the blockers.

## Risks

- Risk: this change spans nearly every persistence table in the core
  orchestration layer, so Diesel schema files, row models, SQL migrations, and
  PostgreSQL adapter write paths can drift out of sync. Severity: high
  Likelihood: high Mitigation: land the migration first, then update
  schema/model files immediately, then fix write paths and targeted lookups
  before running the full test suite.

- Risk: the current PostgreSQL task repository performs a pre-insert duplicate
  issue lookup with no `tenant_id` filter, which will continue to reject
  cross-tenant duplicates even after the unique index becomes tenant-aware.
  Severity: high Likelihood: high Mitigation: update the task repository's
  duplicate check and issue lookup to include `tenant_id` as part of this
  milestone. This is a targeted correction required to make the new constraint
  observable, not the full 1.5.3 scoping pass.

- Risk: the current PostgreSQL backend registry ignores `RequestContext`
  entirely, so duplicate backend names across tenants may insert successfully
  after the migration but remain ambiguous on lookup. Severity: high
  Likelihood: high Mitigation: update backend insert and lookup paths to
  persist and query by `tenant_id` in this milestone. Full RLS remains deferred.

- Risk: the PostgreSQL test template database can mask a new migration if the
  template version is not bumped. Severity: medium Likelihood: high Mitigation:
  add the migration SQL constant to `tests/postgres/helpers.rs`, apply it in
  order, and bump `TEMPLATE_DB` to a new version suffix.

- Risk: adding `tenant_id` to `domain_events` and `audit_logs` may intersect
  with later 1.5.3 work around audit trigger session variables. Severity:
  medium Likelihood: medium Mitigation: keep the migration decision explicit.
  If those tables can be made tenant-aware without forcing premature
  RLS/session-variable work, do so; otherwise document the staged choice in
  `docs/corbusier-design.md` and keep the remaining tightening work clearly
  assigned to 1.5.3.

- Risk: `rstest-bdd` steps silently fail to compile when the scenario entry
  point file is missing. Severity: medium Likelihood: medium Mitigation: if a
  new BDD step directory is created, add a top-level `tests/*_scenarios.rs`
  entry point immediately and verify with `cargo check --tests` or the full
  gate run.

## Progress

- [x] (2026-03-21 00:00Z) Gathered roadmap, design, testing, and existing
  project-memory guidance for roadmap 1.5.2.
- [x] (2026-03-21 00:00Z) Mapped the current migration stack, PostgreSQL test
  harness, and the existing tenant-aware in-memory adapters.
- [x] (2026-03-21 00:00Z) Identified the two PostgreSQL adapter gaps that
  block 1.5.2 success criteria today: task issue pre-checks and backend
  registry lookups remain effectively global.
- [x] (2026-03-22 00:00Z) Added PostgreSQL regression tests for same issue
  reference reuse across tenants, same backend-name reuse across tenants, and
  composite foreign key rejection for tenant-mismatched child rows.
- [x] Stage A: add failing tests for tenant-aware uniqueness and composite
  foreign keys.
- [x] Stage B: add the tenant schema migration and wire it into the PostgreSQL
  template helper.
- [x] Stage C: update Diesel schema/model files and PostgreSQL adapter write
  paths.
- [x] Stage D: add the narrowly scoped repository query filters required to
  make per-tenant uniqueness observable before 1.5.3.
- [x] Stage E: add or extend `rstest-bdd` scenarios for tenant-visible
  behaviour.
- [x] Stage F: update design, user, and roadmap documentation; then run all
  quality gates.

## Surprises & discoveries

- Observation: in-memory task and backend adapters are already tenant-scoped
  via `HashMap<TenantId, ...>`, so the service-level multi-tenant behaviour
  requested by 1.5.2 already exists in memory and can be reused for BDD
  scenarios. Evidence: `src/task/adapters/memory/task.rs` and
  `src/agent_backend/adapters/memory/backend_registry.rs` key all state by
  `TenantId`. Impact: behavioural tests can focus on workflow semantics without
  waiting on PostgreSQL implementation details.

- Observation: the PostgreSQL task repository already wraps operations in
  `with_tenant_tx`, but `find_task_by_issue_ref()` still contains an explicit
  `FIXME(1.5.3)` because it does not filter by `tenant_id`. Evidence:
  `src/task/adapters/postgres/repository.rs`. Impact: 1.5.2 must correct that
  targeted lookup or the new unique index will remain unobservable to callers.

- Observation: the PostgreSQL backend registry still ignores `RequestContext`
  for both writes and reads. Evidence:
  `src/agent_backend/adapters/postgres/repository.rs`. Impact: 1.5.2 must
  persist and query `tenant_id` there, even though the full broad-scoping
  milestone is still 1.5.3.

- Observation: `tests/postgres/helpers.rs` currently builds template database
  `corbusier_test_template_v8`. Evidence: `tests/postgres/helpers.rs`. Impact:
  the implementation must bump the template version or the new schema will be
  skipped by stale template reuse.

- Observation: adding required `tenant_id` to `conversations` forces every raw
  PostgreSQL test helper insert to accept a caller context, otherwise message
  repository tests fail with composite foreign key violations when the stored
  message row uses the request tenant and the synthetic conversation row uses a
  default or missing tenant. Evidence: compile and helper-callsite updates in
  `tests/postgres/helpers.rs` and the message integration tests. Impact:
  tenant-aware helper plumbing is part of the milestone, not optional test
  cleanup.

- Observation: the new `tenants` foreign keys make random `RequestContext`
  tenant IDs invalid until a matching tenant row exists. Evidence: the first
  long-profile gate failed with `conversations_tenant_id_fkey` when helper and
  adapter writes used fresh tenant IDs not present in `tenants`. Impact: 1.5.2
  needs adapter-level tenant bootstrap on write, otherwise the schema change
  breaks the existing request-context contract before tenant lifecycle
  management is introduced.

## Decision log

- Decision: treat 1.5.2 as a schema-and-observability milestone.
  It will add `tenant_id`, replace global uniqueness with tenant-aware
  uniqueness, and add the minimum PostgreSQL lookup changes required so callers
  can observe that new behaviour safely. Rationale: without those targeted
  lookup changes, same-reference and same-name reuse would exist only in the
  raw table constraints, not in the repository behaviour. That would fail the
  roadmap success criteria while still leaving the larger RLS and full
  query-scoping work to 1.5.3. Date/Author: 2026-03-21 / plan author.

- Decision: use a seeded default tenant row in the migration to backfill
  pre-existing rows while keeping the migration deterministic and reversible.
  Rationale: the repository already has historical migrations that created
  tenant-unaware rows. A stable seeded tenant makes the upgrade path safe for
  both existing data and empty test databases. Date/Author: 2026-03-21 / plan
  author.

- Decision: reuse the pattern already proven by
  `migrations/2026-03-11-000000_tenant_scope_mcp_servers/`: add `tenant_id`,
  add `UNIQUE (id, tenant_id)` on parent tables, then replace child foreign
  keys with composite foreign keys. Rationale: that migration already solved
  the same class of tenant-consistency problem for the `tool_registry` bounded
  context. Date/Author: 2026-03-21 / plan author.

- Decision: keep BDD focused on user-observable tenant reuse scenarios, and
  keep raw composite foreign key rejection tests in PostgreSQL integration
  tests. Rationale: database constraint failures are storage-level behaviour,
  not a natural Gherkin workflow. The two styles of test should each prove the
  layer they are best suited to prove. Date/Author: 2026-03-21 / plan author.

- Decision: defer `domain_events` and `audit_logs` tenant columns to 1.5.3.
  Rationale: the current audit trigger and session-variable capture would need
  to be redesigned in the same change to keep those tables coherent, which
  would collapse the boundary between this schema/constraint milestone and the
  later RLS/audit milestone. Date/Author: 2026-03-22 / implementation.

- Decision: lazily provision placeholder `tenants` rows from PostgreSQL write
  adapters and raw PostgreSQL test helpers. Rationale: `RequestContext` already
  treats `TenantId` as an opaque caller-supplied identifier, but the new
  foreign keys now require a concrete parent row. This preserves public API
  stability for 1.5.2 and defers explicit tenant lifecycle management to a
  later milestone. Date/Author: 2026-03-22 / implementation.

## Outcomes & retrospective

Implementation is complete.

Delivered outcomes:

- tenant-owned schema tables now persist `tenant_id`,
- same issue references and backend names can coexist across tenants,
- composite foreign keys reject tenant-mismatched parent/child rows,
- PostgreSQL write paths bootstrap placeholder tenant rows so the new foreign
  keys remain compatible with the existing `RequestContext` contract, and
- `make fmt`, `make check-fmt`, `make lint`, and
  `make test TEST_FLAGS='--profile long --all-targets --all-features'` all
  passed on 2026-03-22.

## Context and orientation

Corbusier is a single Rust crate with bounded contexts under `src/`. The
relevant ones for this milestone are:

- `src/task/` for task lifecycle persistence and issue-origin uniqueness,
- `src/agent_backend/` for backend registration persistence and backend-name
  uniqueness,
- `src/message/` for conversations, messages, agent sessions, handoffs, and
  context snapshots, and
- `src/context/` and `src/tenant/` for the `RequestContext` and tenant
  primitives already delivered by 1.5.1.

The current PostgreSQL schema is built by a migration chain applied in
`tests/postgres/helpers.rs`. The existing files to inspect before changing
anything are:

- `migrations/2026-01-15-000000_create_base_tables/up.sql`
- `migrations/2026-02-03-000000_add_agent_sessions_and_handoffs/up.sql`
- `migrations/2026-02-09-000000_add_tasks_table/up.sql`
- `migrations/2026-02-25-000000_add_backend_registrations_table/up.sql`
- `tests/postgres/helpers.rs`

The current Diesel schema and model files that will need updates are:

- `src/message/adapters/schema.rs`
- `src/message/adapters/models.rs`
- `src/task/adapters/postgres/schema.rs`
- `src/task/adapters/postgres/models.rs`
- `src/agent_backend/adapters/postgres/schema.rs`
- `src/agent_backend/adapters/postgres/models.rs`

The PostgreSQL adapters that must be kept in sync with the new columns and
composite keys are:

- `src/message/adapters/postgres/mod.rs`
- `src/message/adapters/postgres/agent_session/mod.rs`
- `src/message/adapters/postgres/handoff.rs`
- `src/message/adapters/postgres/context_snapshot.rs`
- `src/task/adapters/postgres/repository.rs`
- `src/agent_backend/adapters/postgres/repository.rs`

Existing test infrastructure and helpers:

- `src/test_support.rs` already exposes `test_request_ctx()` and
  `other_tenant_ctx()` under the `test-support` feature.
- `tests/postgres/helpers.rs` owns the migration application order and the
  versioned template database name.
- `tests/postgres/task_tenant_isolation_tests.rs` already proves
  `SET LOCAL app.tenant_id` scoping for task operations.
- `tests/in_memory/backend_registry_tests.rs` and
  `tests/in_memory/task_lifecycle_tests/task_isolation_tests.rs` already prove
  in-memory tenant isolation behaviour.

The parent/child relationships that 1.5.2 must enforce with composite foreign
keys are, at minimum:

Table 1.5.2.1: Composite foreign key relationships enforcing tenant consistency.

```plaintext
conversations(task_id, tenant_id) -> tasks(id, tenant_id)
messages(conversation_id, tenant_id) -> conversations(id, tenant_id)
agent_sessions(conversation_id, tenant_id) -> conversations(id, tenant_id)
handoffs(source_session_id, tenant_id) -> agent_sessions(id, tenant_id)
handoffs(conversation_id, tenant_id) -> conversations(id, tenant_id)
context_snapshots(session_id, tenant_id) -> agent_sessions(id, tenant_id)
context_snapshots(conversation_id, tenant_id) -> conversations(id, tenant_id)
```

If implementation shows that additional child relationships need the same
pattern, add them in the same migration and update this plan.

## Plan of work

### Stage A: write failing tests first

Add regression tests before the migration so the failure mode is explicit.

For the task bounded context, extend PostgreSQL coverage with a test that:

1. creates two `RequestContext` values with distinct `TenantId`s,
2. inserts the same issue-origin task once per tenant, and
3. proves both inserts succeed and each tenant retrieves its own row.

For the agent backend bounded context, add a matching PostgreSQL test that:

1. registers the same backend name once per tenant,
2. proves both registrations succeed, and
3. proves tenant-scoped lookups return the correct registration for the caller.

Add a PostgreSQL constraint test module that inserts parent rows under one
tenant and then uses raw SQL or Diesel inserts to attempt child rows with a
different tenant. Use `#[rstest]` parameterization where possible so one test
body can cover the conversation/task, message/conversation, session/
conversation, handoff/session, and snapshot/session composite foreign keys.

For behaviour tests, either extend existing task/backend tenant feature
coverage or add a new feature with two scenarios:

- two tenants can create tasks from the same issue reference, and
- two tenants can register the same backend name.

Prefer reusing in-memory adapters for these BDD tests because the behaviour is
service-visible and the in-memory implementations already honour tenant
boundaries.

Validation for Stage A: the new PostgreSQL tests must fail before the
implementation, either with unique violations, missing columns, or incorrect
cross-tenant lookups. The new BDD scenarios should compile, even if they fail.

### Stage B: add the migration and wire it into the test template

Create a new migration directory for this milestone, for example
`migrations/2026-03-21-000000_add_tenant_schema_and_constraints/`, containing
`up.sql` and `down.sql`.

In `up.sql`, perform the work in dependency order:

1. Create `tenants` with `id`, `slug`, `name`, `status`, `created_at`, and
   `updated_at`, plus a status check constraint and a unique slug.
2. Insert a deterministic default tenant row for backfill.
3. Add `tenant_id` to tenant-owned tables. At minimum this includes
   `tasks`, `backend_registrations`, `conversations`, `messages`,
   `agent_sessions`, `handoffs`, and `context_snapshots`. Add the column to
   `domain_events` and `audit_logs` if that can be done without dragging the
   milestone into 1.5.3 audit trigger redesign; if not, document the staged
   choice explicitly.
4. Backfill existing rows to the default tenant, then drop any temporary
   default so new writes must supply `tenant_id` explicitly unless a staged
   exception is documented.
5. Add `UNIQUE (id, tenant_id)` to parent tables that will be referenced by
   composite foreign keys.
6. Replace global uniqueness with tenant-aware uniqueness:
   - `tasks`: unique issue-origin index over `(tenant_id, provider,
     repository, issue_number)` for `origin.type = 'issue'`
   - `backend_registrations`: unique index over `(tenant_id, name)`
7. Replace single-column child foreign keys with composite foreign keys using
   `(parent_id, tenant_id)`.
8. Add tenant-aware lookup indexes needed by the existing repositories, such
   as `(tenant_id, task_id)` on `conversations` and
   `(tenant_id, conversation_id, sequence_number)` on `messages`.

In `down.sql`, reverse the above in the opposite dependency order. Recreate the
original global unique indexes and the original single-column foreign keys
before dropping `tenant_id` columns and then dropping `tenants`.

Wire the migration into `tests/postgres/helpers.rs` by:

- adding a new `const` with `include_str!(...)`,
- executing it in `apply_migrations()` after the migrations it depends on, and
- bumping `TEMPLATE_DB` from `corbusier_test_template_v8` to the next version.

Validation for Stage B: the PostgreSQL test helper should build cleanly, and a
fresh template database should include the new tables, indexes, and composite
foreign keys.

### Stage C: update Diesel schema files, row models, and write paths

Once the migration exists, update the Rust-side persistence mapping.

In `src/task/adapters/postgres/schema.rs`,
`src/agent_backend/adapters/postgres/schema.rs`, and
`src/message/adapters/schema.rs`, add the new `tenant_id` columns and update
join declarations to match the new composite relationships. Keep the comment in
`src/message/adapters/schema.rs` accurate if the file is regenerated.

In `src/task/adapters/postgres/models.rs`,
`src/agent_backend/adapters/postgres/models.rs`, and
`src/message/adapters/models.rs`, add `tenant_id` to the row structs and insert
structs wherever the underlying table now requires it.

Update PostgreSQL adapter write paths so new rows always persist the caller's
tenant:

- `PostgresTaskRepository::store()` must write `tenant_id`.
- `PostgresBackendRegistry::register()` must write `tenant_id`.
- Message/conversation/session/handoff/context-snapshot insert helpers must
  populate `tenant_id` on every row they create.

Where composite foreign keys require join or filter adjustments to keep
existing queries working, make those structural changes now. Keep those edits
inside adapters only; do not change domain types or port traits.

Validation for Stage C: the crate compiles, inserts no longer fail on missing
`tenant_id`, and all row conversions still round-trip.

### Stage D: add the minimal tenant-aware lookup fixes required by 1.5.2

This stage is the key boundary-management stage. Only change the lookup paths
that must become tenant-aware for the new per-tenant uniqueness constraints to
behave correctly. Do not turn this into a blanket 1.5.3 query-scoping sweep.

Required lookup corrections:

- In `src/task/adapters/postgres/repository.rs`, make the duplicate issue
  pre-check and the `find_task_by_issue_ref()` SQL query include `tenant_id`.
  Update any branch and pull-request lookups that would otherwise return rows
  across tenants once duplicate external references become legal.
- In `src/agent_backend/adapters/postgres/repository.rs`, include `tenant_id`
  in `register`, `find_by_id`, `find_by_name`, `list_active`, `list_all`, and
  `update` filters so backend reuse across tenants is observable and safe.

If message-level read filters become necessary purely because a new composite
foreign key or tenant-aware join cannot otherwise be expressed, keep those
changes small and document why they are structural for 1.5.2 rather than a full
1.5.3 scoping pass.

Validation for Stage D: the new tenant uniqueness tests pass in PostgreSQL,
same-tenant duplicates still fail, and existing non-tenant tests remain green.

### Stage E: behavioural tests and documentation

Extend or add `rstest-bdd` scenarios to prove the user-visible part of the new
behaviour. A natural fit is:

- one scenario in which tenant A and tenant B both create a task from the same
  issue reference and each sees its own task, and
- one scenario in which tenant A and tenant B both register the same backend
  name and each sees its own registration.

Use the standard BDD (behaviour-driven development) layout:

- `tests/features/<feature>.feature`
- `tests/<feature>_steps/`
- `tests/<feature>_scenarios.rs`

After the code and tests are green, update documentation:

- `docs/corbusier-design.md`: add a dated implementation-decision entry
  summarizing the migration shape, any staged choice for `audit_logs` or
  `domain_events`, and the fact that targeted PostgreSQL lookup filters were
  added here while RLS remains 1.5.3 work.
- `docs/users-guide.md`: explain that issue references and backend names are
  unique per tenant, not globally, and show any updated example usage that a
  developer or operator should know.
- `docs/roadmap.md`: mark 1.5.2 and its sub-bullets as done only after all
  validation commands pass.

Validation for Stage E: behaviour tests pass, documentation reads correctly,
and the roadmap reflects the delivered state.

### Stage F: run the full quality gates

Run the repo's required gates exactly as logged commands with `tee`, using the
long `nextest` profile so PostgreSQL integration tests do not trip the default
5-minute timeout.

Validation for Stage F: every command exits successfully and the final diff
contains only the intended code, migration, test, and documentation changes.

## Concrete steps

Run all commands from the repository root, `/home/user/project`.

1. Review the current migration and adapter state before editing:

   ```bash
   rg -n "tenant_id|FIXME\\(1\\.5\\.3\\)|idx_tasks_issue_origin_unique|idx_backend_registrations_name" \
     src tests migrations docs
   ```

   Expected signal:

   ```plaintext
   src/task/adapters/postgres/repository.rs: ... FIXME(1.5.3) ...
   migrations/2026-02-09-000000_add_tasks_table/up.sql: ... idx_tasks_issue_origin_unique ...
   migrations/2026-02-25-000000_add_backend_registrations_table/up.sql: ... idx_backend_registrations_name ...
   ```

2. Add or extend the failing tests described in Stage A.

3. Create the new migration directory and write `up.sql` and `down.sql`.

4. Update `tests/postgres/helpers.rs` so the migration is applied and the
   template database version is bumped.

5. Update Diesel schema files, row models, and PostgreSQL adapters.

6. Run the Rust quality gates:

   ```bash
   set -o pipefail; make check-fmt 2>&1 | tee /tmp/1-5-2-check-fmt.log
   set -o pipefail; make lint 2>&1 | tee /tmp/1-5-2-lint.log
   set -o pipefail; make test TEST_FLAGS='--profile long --all-targets --all-features' 2>&1 | tee /tmp/1-5-2-test.log
   ```

7. Update documentation and roadmap, then run the documentation gates:

   ```bash
   set -o pipefail; make fmt 2>&1 | tee /tmp/1-5-2-fmt.log
   set -o pipefail; PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 | tee /tmp/1-5-2-markdownlint.log
   set -o pipefail; make nixie 2>&1 | tee /tmp/1-5-2-nixie.log
   ```

8. Inspect the final diff carefully, paying special attention to any unrelated
   markdown churn caused by `make fmt`.

## Validation and acceptance

Acceptance is behavioural and structural.

The feature is done when all of the following are true:

- PostgreSQL permits the same task issue reference in two different tenants,
  but still rejects the same reference twice in one tenant.
- PostgreSQL permits the same backend name in two different tenants, but still
  rejects the same name twice in one tenant.
- Tenant-mismatched child rows fail with foreign key violations rather than
  silently persisting.
- Existing repository and service workflows still pass for the single-tenant
  case.
- Behaviour tests prove the user-visible multi-tenant reuse scenarios.
- Documentation reflects the new tenant-aware behaviour and the roadmap is
  marked done.

Quality criteria:

- Tests:
  - targeted new `rstest` unit/integration coverage for happy and unhappy
    paths,
  - `rstest-bdd` coverage where the workflow is user-visible, and
  - the full workspace suite via the repo's `make test` target.
- Lint and formatting:
  - `make check-fmt`
  - `make lint`
  - `make fmt`
  - `PATH=/root/.bun/bin:$PATH make markdownlint`
  - `make nixie`

The acceptance run should specifically mention the new tenant-aware PostgreSQL
tests in the output summary or via focused filters before the full run.

## Idempotence and recovery

The migration must be safe to apply to an empty database and to an existing
database with pre-tenant rows. The seeded default tenant provides the backfill
anchor for rerunnable local testing.

If the PostgreSQL template database becomes stale, bump the `TEMPLATE_DB`
suffix in `tests/postgres/helpers.rs` before rerunning tests. This is the
expected recovery path; do not debug against a stale template first.

If `make fmt` rewrites unrelated markdown files, restore the unrelated churn
before finalizing. The final diff should include only the migration, adapter,
test, and documentation files relevant to 1.5.2.

## Artifacts and notes

The most useful quick checks during implementation are:

- direct inspection of the new migration in the database template,
- a focused run of the new PostgreSQL tenant tests before the full suite, and
- a final `git diff --stat` review after `make fmt`.

Keep the most relevant evidence in the implementation notes, for example:

```plaintext
PASS postgres_allows_same_issue_reference_in_different_tenants
PASS postgres_allows_same_backend_name_in_different_tenants
PASS composite_fk_rejects_cross_tenant_child_row[message_conversation]
```

## Interfaces and dependencies

The following interfaces should exist unchanged at the end of this work:

- `crate::task::ports::TaskRepository`
- `crate::agent_backend::ports::BackendRegistryRepository`
- `crate::message::ports::repository::MessageRepository`
- `crate::message::ports::agent_session::AgentSessionRepository`
- `crate::message::ports::handoff::AgentHandoffPort`
- `crate::message::ports::context_snapshot::ContextSnapshotPort`
- `crate::context::RequestContext`

The implementation should reuse, not replace, these existing building blocks:

- `crate::context::RequestContext::tenant_id()` as the source of tenant
  identity for persistence writes,
- `crate::message::adapters::postgres::tenant_tx::with_tenant_tx` for
  transaction-scoped tenant context where it is already in use,
- `crate::test_support::{test_request_ctx, other_tenant_ctx}` for multi-tenant
  test setup,
- `tests/postgres/helpers::{ensure_template, TEMPLATE_DB}` for PostgreSQL test
  bootstrapping, and
- the existing in-memory tenant-aware task and backend adapters for BDD
  scenarios.

Do not add a new tenancy abstraction layer. The repository already has the
necessary domain-level tenant identity and context types.

## Revision note

Initial draft created on 2026-03-21. This plan captures the current schema
gaps, the required migration and adapter work for roadmap 1.5.2, and the
explicit boundary that keeps full RLS and broad adapter query scoping assigned
to roadmap 1.5.3.
