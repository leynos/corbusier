# Deliver tool discovery and routing (roadmap 2.1.2)

This ExecPlan (execution plan) is a living document. The sections
`Constraints`, `Tolerances`, `Risks`, `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work
proceeds.

Status: DONE

## Purpose / big picture

Roadmap item 2.1.1 gave Corbusier the ability to register Model Context
Protocol (MCP) servers, manage their lifecycle (start/stop/health), and list
tools exposed by running servers. However, tools discovered from MCP servers
are not persisted -- they exist only as transient query results from the live
process. There is no mechanism to route a tool call by name to the correct
hosting server, no parameter validation against the tool's declared schema, no
policy enforcement before execution, and no audit trail of tool invocations.

After this change, a developer or operator can:

1. Start an MCP server and have its tools automatically persisted in a
   durable catalog that survives process restarts.
2. Query the tool catalog to see all available tools across all registered
   MCP servers, with their schemas and availability status.
3. Execute a tool call by name and have the system resolve which MCP server
   hosts that tool, validate call parameters against the tool's input schema,
   enforce a pluggable policy check, route the call to the correct server, and
   record a complete audit trail of the invocation.
4. Stop an MCP server and have all its tools automatically marked as
   unavailable in the catalog.
5. Have MCP server startup stderr and tool call stderr automatically captured
   and stored via the Rust `object_store` crate, with log references recorded
   in the tool call audit trail. Logs are subject to a configurable rotation
   and retention policy (default: 7-day retention for tool execution logs,
   matching the design document §5.4.2 retention guidance).

Observable success: `make all`, `make markdownlint`, and `make nixie` pass, and
new unit (`rstest`), behavioural (`rstest-bdd`), in-memory integration, and
PostgreSQL integration tests prove happy paths, unhappy paths, and key edge
cases. The roadmap item 2.1.2 and its sub-bullets are marked done.

## Constraints

- Maintain strict hexagonal boundaries within the `tool_registry` bounded
  context: domain code must remain infrastructure-agnostic (no Diesel, no
  process management, no transport details); port traits live in the core
  module; adapters implement ports and do not depend on one another.
- Scope strictly to roadmap 2.1.2: tool discovery, metadata persistence,
  cross-server tool resolution, call routing with schema validation and policy
  enforcement, and audit trail. Do not implement workspace management (2.4.1),
  hook engine execution (2.3.1), or Weaver integration (2.2.1).
- Preserve all existing public behaviour in `message`, `task`,
  `agent_backend`, and the existing `tool_registry` lifecycle service. All
  existing tests must continue to pass without modification.
- Add module-level `//!` docs and public Rustdoc `///` comments (with
  `# Errors` sections) for all new modules and public APIs.
- Avoid `unsafe` code.
- Keep files below the 400-line repository rule by splitting modules early.
- Use `rstest` for unit/integration fixtures and `rstest-bdd` for behavioural
  scenarios.[^1][^2][^3]
- Use existing `pg-embed-setup-unpriv` harness patterns for PostgreSQL
  tests.[^4]
- Use en-GB-oxendict spelling in comments and documentation.[^5]
- The `object_store` crate is an authorized new dependency for this feature
  (user-requested). Use caret version requirement per `AGENTS.md` guidance. No
  other new external crate dependencies without escalation.
- Update documentation as part of the feature: record implementation decisions
  in `docs/corbusier-design.md`, update `docs/users-guide.md` for user-visible
  behaviour, and mark roadmap items done in `docs/roadmap.md` only after all
  quality gates pass.

[^1]: `docs/rust-testing-with-rstest-fixtures.md`
[^2]: `docs/reliable-testing-in-rust-via-dependency-injection.md`
[^3]: `docs/rstest-bdd-users-guide.md`
[^4]: `docs/pg-embed-setup-unpriv-users-guide.md`
[^5]: `docs/complexity-antipatterns-and-refactoring-strategies.md`

## Tolerances (exception triggers)

- Scope: if implementation requires changes to more than 40 files or 3,000 net
  lines, stop and escalate with a reduced-scope option.
- API surface: if implementing 2.1.2 requires incompatible changes to existing
  public APIs outside the `tool_registry` module, stop and escalate.
- Dependencies: the `object_store` crate is pre-authorized for this feature.
  If any additional new external crate is required (e.g., `jsonschema` for full
  JSON Schema validation), stop and escalate with rationale and alternatives.
- Iterations: if a failing test remains unresolved after 4 focused
  fix-and-rerun cycles, stop and document alternatives.
- Milestone duration: if any single implementation stage exceeds 4 hours of
  active work, stop and report remaining unknowns.
- Ambiguity: if multiple valid interpretations exist for tool name collision
  handling (error vs. first-wins) and the choice materially affects downstream
  consumers, stop and present options.

## Risks

- Risk: Adding `call_tool` to the `McpServerHost` trait is a breaking change
  for all implementors. Severity: medium Likelihood: low (only
  `InMemoryMcpServerHost` exists today) Mitigation: update
  `InMemoryMcpServerHost` in the same change; the trait is internal to the
  bounded context.

- Risk: JSON Schema validation without a dedicated crate may be insufficient
  for complex nested schemas. Severity: medium Likelihood: low (most MCP tools
  use flat object schemas with required fields) Mitigation: implement
  lightweight validation (required fields + object type check) in a domain
  validation module. This can be replaced with the `jsonschema` crate later
  without changing the port contract.

- Risk: Ambiguous tool names across servers could cause routing confusion.
  Severity: medium Likelihood: medium Mitigation: enforce a unique index on
  `tool_name` in the catalog table. If a second server registers a tool with a
  name already in the catalog, the discover operation returns an error. This is
  explicit and safe; first-wins semantics can be added later if needed.

- Risk: New PostgreSQL migration may be omitted from
  `tests/postgres/helpers.rs`, causing false negatives. Severity: medium
  Likelihood: medium Mitigation: add the migration SQL constant and apply call
  alongside existing constants in `apply_migrations()`, following the
  established pattern.

- Risk: The `object_store` crate adds compile-time cost and a new
  transitive dependency tree. Severity: low Likelihood: low (crate is
  well-maintained, part of the Apache Arrow ecosystem, and designed for this
  exact use case) Mitigation: use only the `object_store` base crate with the
  local filesystem backend (no cloud SDKs). The crate's `LocalFileSystem`
  implementation is lightweight. Cloud backends can be feature-gated later.

- Risk: Stderr log volume from chatty MCP servers could consume excessive
  storage if retention or rotation is misconfigured. Severity: medium
  Likelihood: medium Mitigation: enforce a per-log-file size cap (default 10
  MiB), a per-server maximum retained log count (default 100), and a time-based
  retention period (default 7 days). The rotation policy is configurable and
  the retention sweep runs at server start and on a periodic schedule.

- Risk: File count and line count may approach tolerance limits given the
  number of new domain types, ports, adapters, services, and tests. Severity:
  low Likelihood: medium Mitigation: keep types small (one concern per file),
  extract early, and track totals at each stage checkpoint.

## Progress

- [x] Gathered roadmap and design requirements for 2.1.2.
- [x] Mapped existing hexagonal module, test, and adapter patterns.
- [x] Authored initial ExecPlan draft.
- [x] Stage A: domain types, port traits, and compilation checkpoint.
- [x] Stage B: adapter implementations and migration.
- [x] Stage C: service implementation with tests (red then green).
- [x] Stage D: integration and BDD tests.
- [x] Stage E: documentation, roadmap update, and final gates.

## Surprises & discoveries

- PostgreSQL integration tests failed on first run because the template
  database (`corbusier_test_template`) had been created by a prior test run
  before the tool catalog migration was added to `apply_migrations()`. The
  `ensure_template_exists` function checks for existence and returns early, so
  the stale template (missing `mcp_tool_catalog`, `tool_call_audit_log`, and
  `tool_log_metadata` tables) was reused. Fix: introduced a versioned template
  name (`corbusier_test_template_v2`) and added a doc comment explaining to
  bump the suffix whenever migrations are added.

- The `McpServerHost::start` return type change from `()` to
  `StartHostResult` required updating the lifecycle service's `start` method
  call site. This was straightforward because `InMemoryMcpServerHost` is the
  only implementor, but it highlights the importance of noting breaking trait
  changes in the plan.

- Strict clippy lints (`indexing_slicing`, `shadow_unrelated`) caught
  several test code patterns that needed attention. Using
  `#[expect(clippy::indexing_slicing, reason = "...")]` per-function (following
  the pattern in `conversation_flow_tests.rs`) resolved these cleanly.

- The `object_store` crate's `InMemory` backend worked seamlessly for
  testing, confirming the decision to use `object_store` as the storage
  abstraction. The `PutPayload::from(bytes)` API made storing `Bytes` values
  trivial.

## Decision log

- Decision: introduce `ToolDiscoveryRoutingService` as a sibling service to
  `McpServerLifecycleService` rather than extending the existing service.
  Rationale: the lifecycle service manages server state transitions; the
  discovery/routing service manages tool catalog and call routing. These are
  distinct responsibilities. Composing them at the call site (caller invokes
  lifecycle service then discovery service) keeps each service focused and
  testable. Date/Author: 2026-03-04 / plan author.

- Decision: use composition rather than coupling for lifecycle-to-discovery
  integration. When a server starts, the caller invokes
  `discover_and_persist_tools()`; when it stops, the caller invokes
  `mark_tools_unavailable()`. The services share port instances but do not
  reference each other. Rationale: avoids circular dependencies, simplifies
  testing, and matches the existing pattern where the lifecycle service does
  not know about consumers of its state changes. Date/Author: 2026-03-04 / plan
  author.

- Decision: enforce unique tool names across all servers via a database unique
  index. If two servers advertise the same tool name, the second discovery
  attempt returns `AmbiguousToolName`. Rationale: routing by tool name requires
  unambiguous resolution. A first-wins or priority-based strategy adds
  complexity without clear requirements. The explicit error is safe and can be
  relaxed later. Date/Author: 2026-03-04 / plan author.

- Decision: implement lightweight schema validation (required fields + object
  type check) rather than full JSON Schema validation via a crate dependency.
  Rationale: avoids adding a new crate (tolerance constraint), covers the
  common case (MCP tools use flat object schemas), and the validation module
  can be swapped for a crate-backed implementation later without changing any
  port contract. Date/Author: 2026-03-04 / plan author.

- Decision: provide `AllowAllPolicy` as the default `ToolPolicyEnforcer`
  adapter. The policy port is an extensibility point; real authorization will
  be implemented when the workspace and user permission systems exist.
  Rationale: delivers the enforcement architecture without blocking on
  unimplemented bounded contexts. Date/Author: 2026-03-04 / plan author.

- Decision: defer stream events (`ToolCallInitiated`, `ToolExecutionComplete`)
  and full `ToolExecutionContext` (workspace, task, conversation IDs). These
  depend on infrastructure and bounded contexts that do not exist yet.
  Rationale: the routing backbone and audit trail provide the essential
  observability for 2.1.2. Events can be wired in when the event infrastructure
  is built. Date/Author: 2026-03-04 / plan author.

- Decision: use the Rust `object_store` crate with its `LocalFileSystem`
  backend for stderr log storage. Define a `ToolLogStore` port trait that wraps
  `object_store` operations behind the hexagonal boundary. Rationale:
  `object_store` provides a unified API across local filesystem, S3, GCS, and
  Azure Blob Storage. Starting with `LocalFileSystem` keeps the implementation
  simple and testable (including an `InMemory` backend for tests). When the
  project needs cloud storage (referenced in the design doc §1824-1828 as a
  future capability), the adapter can be swapped without changing the port
  contract. The crate is maintained by the Apache Arrow project and is widely
  used in the Rust ecosystem. Date/Author: 2026-03-04 / plan author.

- Decision: capture stderr from two lifecycle points: (a) MCP server startup
  (the `start` operation on `McpServerHost`), and (b) individual tool calls
  (the `call_tool` operation). Startup logs capture the server's initialization
  output; tool call logs capture per-invocation diagnostic output. Both are
  stored as opaque byte blobs in `object_store` with structured path keys, and
  references (object store paths) are recorded in the principal audit log.
  Rationale: stderr is the standard diagnostic channel for MCP servers.
  Capturing it at both lifecycle points gives operators full visibility into
  server behaviour without requiring the MCP server itself to implement
  structured logging. Date/Author: 2026-03-04 / plan author.

- Decision: implement log rotation with three configurable parameters:
  `max_bytes_per_log` (default 10 MiB, truncates with a trailing marker),
  `max_logs_per_server` (default 100, oldest deleted first), and
  `retention_period` (default 7 days, aligned with design doc §5.4.2 retention
  for tool execution logs at DEBUG level). Retention sweeps run at server start
  and can be triggered explicitly. Rationale: prevents unbounded storage growth
  while keeping sufficient history for debugging. The 7-day default matches the
  design document's retention guidance for tool execution logs. All parameters
  are configurable to allow operators to tune for their environment.
  Date/Author: 2026-03-04 / plan author.

- Decision: use `object_store::memory::InMemory` as the test backend for
  `ToolLogStore`, avoiding filesystem side-effects in unit and integration
  tests. Rationale: `object_store` ships with a first-party `InMemory`
  implementation that is API-compatible with `LocalFileSystem`. This eliminates
  temp-directory management in tests and makes assertions on stored log content
  trivial. Date/Author: 2026-03-04 / plan author.

## Outcomes & retrospective

### Acceptance evidence

All 11 acceptance criteria verified:

1. Tool discovery and persistence: verified by PostgreSQL integration test
   `catalog_survives_service_reconstruction` -- tools discovered from a running
   server are persisted and survive service reconstruction from the same
   connection pool.
2. Cross-server routing: verified by in-memory integration test
   `two_servers_route_correctly` and BDD scenario "Route a tool call to the
   correct server".
3. Schema validation: verified by unit test
   `call_tool_schema_validation_failure`.
4. Policy enforcement: verified by unit test `call_tool_policy_denied`
   using `DenyAllPolicy`.
5. Audit trail: verified by PostgreSQL integration test
   `audit_log_persisted` and BDD scenario audit assertions.
6. Tool unavailability after stop: verified by BDD scenario "Tool becomes
   unavailable when server stops" and integration test
   `tool_unavailable_after_stop`.
7. Unknown tool error: verified by BDD scenario "Unknown tool call is
   rejected".
8. Stderr capture for tool calls: verified by BDD scenario "Tool call
   stderr is captured in the log store" and unit test
   `call_tool_captures_stderr_in_log_store`.
9. Startup stderr capture: verified by unit test
   `store_startup_stderr_captures_and_sweeps`.
10. Log rotation (`max_logs_per_server`): verified by unit test
    `store_startup_stderr_captures_and_sweeps` (overflow sweep) and
    integration test `log_rotation_enforces_max_count`.
11. Stderr truncation: verified by unit test
    `stderr_truncation_at_max_bytes`.

### Quality gates

- `make all`: all 646 tests pass, lint clean, format clean.
- `make markdownlint`: pass.
- `make nixie`: pass.

### Metrics

- New files: 24 (domain types, ports, adapters, migration, tests, BDD).
- Modified files: 16 (existing adapters, services, test harness, docs).
- Net lines added: approximately 2,800 (within 3,000-line tolerance).
- Test count increase: 14 new tests (632 to 646).

### Retrospective

What went well:

- The hexagonal architecture made the new service straightforward to
  compose: each port trait was tested independently, then the service wired
  them together. The `InMemory` adapters provided fast, deterministic unit
  tests.
- The existing test harness patterns (template DB cloning, rstest fixtures,
  BDD step definitions) were easy to follow and extend.
- The `object_store` crate was a good choice for log storage -- the
  `InMemory` backend eliminated filesystem side-effects in tests.

What could be improved:

- The template database versioning scheme (`_v2` suffix) is fragile. A
  hash-based approach (e.g., hashing migration file contents) would be more
  robust. Consider this for a future improvement.
- The plan's Stage C was the largest stage and could benefit from being
  split into two sub-stages (service implementation + unit tests) in future
  plans of similar scope.

## Context and orientation

Corbusier is a Rust application following hexagonal architecture. Each bounded
context lives under `src/<context>/` with four layers: `domain/` (value
objects, aggregates, error types), `ports/` (async trait contracts),
`adapters/` (in-memory and PostgreSQL implementations), and `services/`
(orchestration).

The `tool_registry` bounded context (`src/tool_registry/`) was established by
roadmap 2.1.1 and provides:

- **Domain types** in `src/tool_registry/domain/`: `McpServerId` and
  `McpServerName` (identity), `McpServerRegistration` (aggregate root with
  lifecycle state machine), `McpServerLifecycleState` (`Registered`, `Running`,
  `Stopped`), `McpServerHealthSnapshot`, `McpTransport` (stdio/HTTP+SSE
  config), `McpToolDefinition` (tool name, description, input/output schemas as
  `serde_json::Value`), and `ToolRegistryDomainError`.

- **Port traits** in `src/tool_registry/ports/`: `McpServerRegistryRepository`
  (CRUD for server registrations) and `McpServerHost` (runtime lifecycle:
  `start`, `stop`, `health`, `list_tools`).

- **Adapters** in `src/tool_registry/adapters/`: `InMemoryMcpServerRegistry`
  (memory repository), `PostgresMcpServerRegistry` (Diesel-backed), and
  `InMemoryMcpServerHost` (deterministic test double tracking running servers,
  health states, and tool catalogs).

- **Service** in `src/tool_registry/services/lifecycle/`:
  `McpServerLifecycleService<R, H, C>` orchestrating register, start, stop,
  refresh_health, list_all, find_by_name, and list_tools operations with
  compensation logic for failed persistence after host side-effects.

- **Migration**: `migrations/2026-02-28-000000_add_mcp_servers_table/` creates
  the `mcp_servers` table.

Tests follow established patterns:

- Unit tests: `rstest` fixtures, `#[tokio::test(flavor = "multi_thread")]`.
- In-memory integration: `tests/in_memory/mcp_server_lifecycle_tests.rs`,
  registered in `tests/in_memory.rs`.
- PostgreSQL integration: `tests/postgres/mcp_server_lifecycle_tests.rs`,
  registered in `tests/postgres.rs`, using template DB cloning via
  `tests/postgres/helpers.rs`.
- BDD: `tests/features/mcp_server_lifecycle.feature` with step definitions in
  `tests/mcp_server_lifecycle_steps.rs` using `rstest_bdd_macros`.

Build gates (from `AGENTS.md` and `Makefile`):

- `make check-fmt` -- `cargo fmt --all -- --check`
- `make lint` -- `cargo doc --no-deps` + `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`
- `make test` -- `RUSTFLAGS="-D warnings" cargo nextest run --all-targets
  --all-features`
- `make fmt` -- formatting (code + markdown)
- `make markdownlint` -- markdown linting
- `make nixie` -- Mermaid diagram validation

Design references:

- `docs/corbusier-design.md` §2.2.4: requirement matrix for F-005-RQ-002
  (tool discovery) and F-005-RQ-003 (tool execution routing).
- `docs/corbusier-design.md` §6.1.4: architecture diagram showing Tool
  Registry Core, MCP Server Management, Tool Execution, and Built-in Tools
  subgraphs; design types `ToolDefinition`, `ToolProvider` trait,
  `ToolExecutionContext`.
- `docs/corbusier-design.md` §4.4.2.2: three-layer tool call validation
  (schema, authorization, runtime).
- `docs/corbusier-design.md` §6.4.2.4: policy enforcement points with
  `PolicyEnforcementPoint` trait and `EnforcementResult` enum.

## Plan of work

### Stage A: Domain types, port traits, and compilation checkpoint

Create the new domain types and port traits that define the 2.1.2 contracts. No
service logic, no adapters, no tests that exercise behaviour yet -- only types
and traits that compile.

In `src/tool_registry/domain/`, create six new files:

`catalog.rs` defines `CatalogEntryId` (UUID newtype) and `CatalogEntry`
(aggregate linking a tool to its hosting server). `CatalogEntry` holds
`id: CatalogEntryId`, `server_id: McpServerId`, `server_name: McpServerName`,
`tool: McpToolDefinition`, `available: bool`, `discovered_at: DateTime<Utc>`,
and `updated_at: DateTime<Utc>`. Constructors:
`new(server_id, server_name, tool, clock)` (creates with `available: true`).
Mutations: `mark_available(clock)`, `mark_unavailable(clock)`,
`update_tool(tool, clock)`. Accessors for all fields.

`routing.rs` defines the tool call lifecycle types with timing separated from
outcomes:

- `ToolCallId` (UUID newtype) uniquely identifies an invocation.
- `ToolCallRequest` holds `call_id: ToolCallId`, `tool_name: String`,
  `parameters: Value`, and `initiated_at: DateTime<Utc>`.
  `ToolCallRequest::new(tool_name, parameters, clock)` generates a fresh
  `ToolCallId` and captures the current timestamp.
- `ToolCallOutcome` is an enum: `Success { content: Value }` or
  `Failure { error: String }`. Convenience methods `is_success()` and
  `is_failure()`.
- `ToolCallTiming` carries `duration: std::time::Duration` and
  `completed_at: DateTime<Utc>`, computed after the host call returns.
- `ToolCallResult` combines routing and timing metadata: `call_id`, `tool_name`,
  `server_id: McpServerId`, `outcome: ToolCallOutcome`,
  `duration: std::time::Duration`, and `completed_at: DateTime<Utc>`.
  Constructed via
  `ToolCallResult::from_request(request, server_id, outcome, timing)` which
  copies fields from the request and timing structs.

`audit.rs` defines `ToolCallAuditRecord` holding `id: Uuid`,
`call_id: ToolCallId`, `tool_name: String`, `server_id: McpServerId`,
`parameters: Value`, `outcome: ToolCallOutcome`, `duration: Duration`,
`initiated_at`, `completed_at`, and `stderr_log_path: Option<String>` (the
object store path where captured stderr is stored; `None` when no stderr was
captured or storage failed). Constructor `from_result(result, parameters)`;
builder method `with_stderr_log_path(path)` to attach the log reference.

`policy.rs` defines `PolicyDecision` enum (`Allow`, `Deny { reason: String }`).
`PolicyDecision::is_allowed()` and `denial_reason()` accessors.

`validation.rs` defines
`validate_parameters(input_schema: &Value, parameters: &Value) -> Result<(), ToolRegistryDomainError>`.
 This function checks: (a) if `input_schema` has `"type": "object"`,
`parameters` must be a JSON object; (b) if `input_schema` has a `"required"`
array, every listed key must be present in `parameters`. Returns
`ToolRegistryDomainError::SchemaValidationFailed` on failure.

`log_capture.rs` defines domain types for stderr log capture and retention:

`LogEntryId` (UUID newtype) uniquely identifies a captured log blob.
`LogEntryKind` is an enum with two variants: `ServerStartup` (stderr captured
during `start`) and `ToolCall { call_id: ToolCallId }` (stderr captured during
a specific tool call invocation). `LogEntryMetadata` holds `id: LogEntryId`,
`server_id: McpServerId`, `kind: LogEntryKind`, `object_path: String` (the key
in `object_store`), `byte_count: u64`, `captured_at: DateTime<Utc>`, and
`expires_at: DateTime<Utc>` (computed from retention policy at capture time).
`LogCaptureContext` bundles a `clock: &dyn Clock` and
`retention: &LogRetentionPolicy` for log capture operations. Constructors:
`for_startup(server_id, byte_count, ctx: &LogCaptureContext)` and
`for_tool_call(server_id, call_id, byte_count, ctx: &LogCaptureContext)`. The
object path is generated internally from the server ID, kind, and entry ID.

`LogRetentionPolicy` holds three configurable parameters:
`max_bytes_per_log: u64` (default 10 MiB = `10 * 1024 * 1024`),
`max_logs_per_server: usize` (default 100), and
`retention_period: chrono::Duration` (default 7 days). Implements `Default`
with these values. Provides
`is_expired(&self, entry: &LogEntryMetadata, now: DateTime<Utc>) -> bool`.

The object store path convention is:
`tool_logs/{server_id}/{kind}/{log_entry_id}.stderr`

where `{kind}` is `startup` or `call/{call_id}`.

In `src/tool_registry/domain/error.rs`, add new variants to
`ToolRegistryDomainError`: `ToolNotFound(String)`,
`ToolUnavailable { tool_name: String, server_id: McpServerId }`,
`SchemaValidationFailed { tool_name: String, reason: String }`,
`PolicyDenied { tool_name: String, reason: String }`,
`ToolCallTimeout { tool_name: String, call_id: ToolCallId }`, and
`AmbiguousToolName { tool_name: String, server_count: usize }`.

In `src/tool_registry/domain/mod.rs`, add `mod` declarations for the six new
files (`audit`, `catalog`, `log_capture`, `policy`, `routing`, `validation`)
and `pub use` re-exports for all new public types.

In `src/tool_registry/ports/`, create three new files:

`catalog.rs` defines the `ToolCatalogRepository` async trait with methods:
`sync_server_tools(server_id, entries)` to persist/update a batch of catalog
entries for a server; `mark_server_tools_unavailable(server_id)` and
`mark_server_tools_available(server_id)` to toggle availability for all of a
server's tools; `find_by_tool_name(tool_name)` to resolve a tool to its catalog
entry (returning available entries preferentially); `list_all()` to return the
complete catalog; and `record_audit(record)` to persist an audit trail entry.
Define `ToolCatalogError` (variants: `DuplicateEntry(CatalogEntryId)`,
`NotFound(String)`, `InvalidPersistedData(Arc<dyn Error + Send + Sync>)`,
`Persistence(Arc<dyn Error + Send + Sync>)`) and `ToolCatalogResult<T>` type
alias.

`policy.rs` defines the `ToolPolicyEnforcer` async trait with one method:
`evaluate(tool_name, parameters) -> Result<PolicyDecision, ToolPolicyError>`.
Define `ToolPolicyError` with a single variant
`EvaluationFailed(Arc<dyn Error + Send + Sync>)` and `ToolPolicyResult<T>` type
alias.

`log_store.rs` defines the `ToolLogStore` async trait -- the hexagonal port
wrapping `object_store` operations. Methods:

`store_log(&self, metadata, content: bytes::Bytes, retention: &LogRetentionPolicy)`
 `-> ToolLogStoreResult<()>` -- writes a log blob to the object store at
`metadata.object_path()`. The `retention` parameter provides
`max_bytes_per_log`; if content exceeds that limit, the implementation
truncates at the byte boundary and appends a marker line
`\n--- truncated at {max_bytes_per_log} bytes ---\n`.

`retrieve_log(&self, path: &str) -> ToolLogStoreResult<bytes::Bytes>` -- reads
a log blob by path.

`delete_log(&self, path: &str) -> ToolLogStoreResult<()>` -- deletes a single
log blob.

`list_logs_for_server(&self, server_id: McpServerId) -> ToolLogStoreResult<Vec<String>>`
 -- lists all log blob paths for a server by prefix scan on
`tool_logs/{server_id}/`.

`sweep_expired(&self, server_id: McpServerId, ctx: &SweepContext<'_>) -> ToolLogStoreResult<usize>`
 -- deletes logs that are past their `expires_at` or exceed
`max_logs_per_server` (oldest first). `SweepContext` bundles the policy,
wall-clock timestamp, and entry metadata. Returns the count of deleted entries.

Define `ToolLogStoreError` with variants: `StoreFailed(String)`,
`RetrieveFailed(String)`, `DeleteFailed(String)`, `ListFailed(String)`. All
wrap the underlying `object_store::Error` description. Define
`ToolLogStoreResult<T>` type alias.

The `ToolLogStore` port does not expose `object_store` types in its API -- the
domain boundary uses `bytes::Bytes` (which `object_store` already depends on)
and domain types (`LogEntryMetadata`, `McpServerId`). This keeps the port
infrastructure-agnostic: tests use the in-memory backend, production uses
`LocalFileSystem`, and a future cloud adapter uses S3/GCS without changing the
port.

In `src/tool_registry/ports/host.rs`, add `call_tool` to the `McpServerHost`
trait: `async fn call_tool(&self, ctx, server, request: &ToolCallRequest)`
`-> McpServerHostResult<ToolCallHostResult>` where `ToolCallHostResult` is a
new struct holding `content: Value` and `stderr_output: Option<bytes::Bytes>`
(the captured stderr from the tool call, if any). Similarly, extend the return
type of `start` to `McpServerHostResult<StartHostResult>` where
`StartHostResult` holds `stderr_output: Option<bytes::Bytes>` (captured startup
stderr). Define these result structs in `host.rs`.

Add two new variants to `McpServerHostError`:
`ToolCallFailed { server_id: McpServerId, tool_name: String, reason: String }`
and `ToolCallTimeout { server_id: McpServerId, tool_name: String }`.

In `src/tool_registry/ports/mod.rs`, add `mod catalog;`, `mod policy;`, and
`mod log_store;` with corresponding `pub use` re-exports.

Go/no-go checkpoint: run `make check-fmt` and `make lint`. Both must pass
before proceeding. The `InMemoryMcpServerHost` in
`src/tool_registry/adapters/runtime.rs` will fail compilation because it does
not yet implement `call_tool` -- add a `todo!()` stub to unblock the lint gate,
clearly marked with a `// TODO(2.1.2): implement in Stage B` comment.

### Stage B: Adapter implementations and migration

Implement all adapter code: in-memory catalog, allow-all policy, in-memory host
`call_tool` and startup stderr, log store adapters, Postgres catalog adapter,
and the database migration.

In `Cargo.toml`, add the `object_store` dependency under `[dependencies]`:

```toml
# Object storage for tool stderr log capture
object_store = "0.12.0"
bytes = "1.10.1"
```

The `bytes` crate is already a transitive dependency (via `object_store`,
`diesel`, `tokio`) but must be declared as a direct dependency since the port
API uses `bytes::Bytes`.

In `src/tool_registry/adapters/memory/catalog.rs`, implement
`InMemoryToolCatalog` with internal state
`Arc<RwLock<InMemoryToolCatalogState>>` containing a
`HashMap<String, CatalogEntry>` (keyed by tool name) and a
`Vec<ToolCallAuditRecord>` for audit records. Implement
`ToolCatalogRepository`. Add a test helper method
`audit_records(&self) -> Vec<ToolCallAuditRecord>` for test assertions.

In `src/tool_registry/adapters/memory/mod.rs`, add `mod catalog;` and
`pub use catalog::InMemoryToolCatalog;`.

In `src/tool_registry/adapters/policy.rs`, implement `AllowAllPolicy`
(zero-field struct, `#[derive(Debug, Clone, Default)]`) implementing
`ToolPolicyEnforcer` by always returning `PolicyDecision::Allow`. Also
implement `DenyAllPolicy` (for test use) that always returns
`PolicyDecision::Deny { reason }`.

In `src/tool_registry/adapters/mod.rs`, add `mod policy;` and
`pub use policy::{AllowAllPolicy, DenyAllPolicy};`.

In `src/tool_registry/adapters/log_store.rs`, implement two `ToolLogStore`
adapters:

`ObjectStoreLogAdapter` wraps an `Arc<dyn object_store::ObjectStore>` and
implements `ToolLogStore` by delegating to the underlying store. The
`store_log` method enforces truncation via
`LogRetentionPolicy::max_bytes_per_log` before writing. The
`list_logs_for_server` method uses `ObjectStore::list` with the prefix
`object_store::path::Path::from(format!("tool_logs/{server_id}/"))` and
reconstructs `LogEntryMetadata` from the stored path convention. The
`sweep_expired` method lists entries, filters by `expires_at < now` and count >
`max_logs_per_server`, and deletes the excess.

The constructor `ObjectStoreLogAdapter::new(store: Arc<dyn ObjectStore>)`
accepts any `ObjectStore` implementation. Factory helpers:

`ObjectStoreLogAdapter::in_memory() -> Self` creates an
`object_store::memory::InMemory`-backed adapter for tests. Production callers
use `ObjectStoreLogAdapter::new(store)` with a suitable `ObjectStore` backend.

In `src/tool_registry/adapters/mod.rs`, add `mod log_store;` and
`pub use log_store::ObjectStoreLogAdapter;`.

In `src/tool_registry/adapters/runtime.rs`, extend `InMemoryHostState` with a
`tool_call_results: HashMap<(McpServerName, String), Value>` field and a
`tool_call_stderr: HashMap<(McpServerName, String), bytes::Bytes>` field for
simulating stderr capture. Add test helpers
`set_tool_call_result(&self, server_name, tool_name, result)` and
`set_tool_call_stderr(&self, server_name, tool_name, stderr)`. Add
`startup_stderr: HashMap<McpServerName, bytes::Bytes>` and helper
`set_startup_stderr(&self, server_name, stderr)`.

Update `start` to return `StartHostResult` with the configured startup stderr
(or `None`). Implement `call_tool`: check server is running (else
`NotRunning`), look up the result by
`(server.name().clone(), tool_name.to_owned())`, return `ToolCallHostResult`
with the value and any configured stderr, or `ToolCallFailed` if no result is
configured. Remove the `todo!()` stub from Stage A.

Create migration directory
`migrations/2026-03-04-000000_add_tool_catalog_tables/` with `up.sql` and
`down.sql`:

`up.sql` creates table `mcp_tool_catalog` with columns: `id UUID PRIMARY KEY`,
`server_id UUID NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE`,
`server_name VARCHAR(100) NOT NULL`, `tool_name VARCHAR(255) NOT NULL`,
`tool_description TEXT NOT NULL`, `input_schema JSONB NOT NULL`,
`output_schema JSONB`, `available BOOLEAN NOT NULL DEFAULT true`,
`discovered_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`,
`updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`. Adds unique index
`idx_mcp_tool_catalog_tool_name` on `tool_name`, index on `server_id`, index on
`(available, tool_name)`, and an `updated_at` trigger.

Also creates table `tool_call_audit_log` with columns: `id UUID PRIMARY KEY`,
`call_id UUID NOT NULL`, `tool_name VARCHAR(255) NOT NULL`,
`server_id UUID NOT NULL`, `parameters JSONB NOT NULL`,
`outcome VARCHAR(50) NOT NULL CHECK (outcome IN ('success', 'failure'))`,
`outcome_content JSONB`, `outcome_error TEXT`, `duration_ms BIGINT NOT NULL`,
`initiated_at TIMESTAMPTZ NOT NULL`, `completed_at TIMESTAMPTZ NOT NULL`,
`stderr_log_path VARCHAR(512)` (nullable -- path reference into the object
store where captured stderr is stored; NULL if no stderr was captured). Adds
indexes on `call_id`, `tool_name`, and `initiated_at`.

Also creates table `tool_log_metadata` with columns: `id UUID PRIMARY KEY`,
`server_id UUID NOT NULL REFERENCES mcp_servers(id) ON DELETE CASCADE`,
`kind VARCHAR(50) NOT NULL CHECK (kind IN ('startup', 'tool_call'))`,
`call_id UUID` (nullable -- populated only when `kind = 'tool_call'`),
`object_path VARCHAR(512) NOT NULL`, `byte_count BIGINT NOT NULL`,
`captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`,
`expires_at TIMESTAMPTZ NOT NULL`. Adds indexes on `(server_id, kind)`,
`expires_at`, and a unique index on `object_path`. This table is the
authoritative index of all stderr log blobs stored in `object_store`; the
actual bytes live in the object store, and this table tracks metadata and
expiry for retention sweeps.

`down.sql` drops the trigger, function, and all three tables.

In `src/tool_registry/adapters/postgres/`, create three new files:

`catalog_schema.rs` with Diesel `table!` macros for `mcp_tool_catalog`,
`tool_call_audit_log`, and `tool_log_metadata`.

`catalog_models.rs` with `CatalogEntryRow` (Queryable/Selectable),
`NewCatalogEntryRow` (Insertable), `AuditLogRow` (Queryable), `NewAuditLogRow`
(Insertable), `LogMetadataRow` (Queryable), and `NewLogMetadataRow`
(Insertable) structs, plus conversion functions to/from domain types.

`catalog_repository.rs` with `PostgresToolCatalog` implementing
`ToolCatalogRepository`. The `record_audit` method accepts an optional
`stderr_log_path` to store in the `tool_call_audit_log.stderr_log_path` column.
Follows the `spawn_blocking` pattern from `PostgresMcpServerRegistry`. Uses the
pool type `Pool<ConnectionManager<PgConnection>>`.

`log_metadata_repository.rs` with `PostgresLogMetadataRepository` providing
methods to insert, list by server, delete by ID, and query expired entries.
This is used by the `ToolDiscoveryRoutingService` to coordinate retention
sweeps (the service reads metadata from Postgres, then deletes blobs from
`object_store`, then removes the metadata rows).

In `src/tool_registry/adapters/postgres/mod.rs`, add module declarations and
re-exports for `PostgresToolCatalog`.

In `tests/postgres/helpers.rs`, add `ADD_TOOL_CATALOG_SQL` constant using
`include_str!` and add the `batch_execute` call to `apply_migrations()`.

Go/no-go checkpoint: run `make check-fmt` and `make lint`. Both must pass. No
tests exercise the new code yet, so `make test` should still pass with the
existing test count.

### Stage C: Service implementation with unit tests

Create the `ToolDiscoveryRoutingService` and its unit tests.

In `src/tool_registry/services/discovery/mod.rs`, define a
`ServicePorts<Cat, Reg, H, Pol, Log>` struct grouping the five port
dependencies as `Arc` references (`catalog`, `registry`, `host`, `policy`,
`log_store`). Then define
`ToolDiscoveryRoutingService<Cat, Reg, H, Pol, Log, C>` parameterized over the
same port types plus `Clock`. The constructor
`new(ports: ServicePorts<...>, retention_policy: LogRetentionPolicy, clock: Arc<C>)`
 destructures the grouped ports into individual fields. Internally the service
stores each `Arc` field plus a `LogRetentionPolicy` value and an `Arc<C>` clock.

Public methods:

`discover_and_persist_tools(&self, server_id) -> ToolDiscoveryRoutingServiceResult<Vec<CatalogEntry>>`:
 load server from registry (fail with `NotFound` if absent), verify lifecycle
state allows tool queries, call `host.list_tools()` to get tool definitions,
map each to a `CatalogEntry`, call `catalog.sync_server_tools()` to persist,
return the entries.

`mark_tools_unavailable(&self, server_id) -> ToolDiscoveryRoutingServiceResult<()>`:
 delegate to `catalog.mark_server_tools_unavailable(server_id)`.

`mark_tools_available(&self, server_id) -> ToolDiscoveryRoutingServiceResult<()>`:
 delegate to `catalog.mark_server_tools_available(server_id)`.

`list_catalog(&self) -> ToolDiscoveryRoutingServiceResult<Vec<CatalogEntry>>`:
delegate to `catalog.list_all()`.

`call_tool(&self, request: ToolCallRequest) -> ToolDiscoveryRoutingServiceResult<ToolCallResult>`:
 the core routing method. Flow:

1. Resolve: `catalog.find_by_tool_name(request.tool_name())`. Fail with
   `ToolNotFound` if `None`.
2. Availability: check `entry.available()`. Fail with `ToolUnavailable` if
   false.
3. Schema validation: call `validate_parameters(entry.tool().input_schema(),
   request.parameters())`. Fail with `SchemaValidationFailed` on error.
4. Policy enforcement: call `policy.evaluate(request.tool_name(),
   request.parameters())`. Fail with `PolicyDenied` if denied.
5. Runtime check: load server from registry, verify lifecycle is `Running`.
   Fail with `ToolUnavailable` if not.
6. Execute: call `host.call_tool(server, tool_name, parameters)`. Wrap in
   `tokio::time::timeout` with 30-second default. The host returns a
   `ToolCallHostResult` containing the content `Value` and an optional
   `stderr_output: Option<bytes::Bytes>`.
7. Stderr capture: if `host_result.stderr_output` is `Some(bytes)` and
   non-empty, store the log:
   - Build a `LogCaptureContext` carrying the clock reference, the retention
     policy reference, and the `tenant_id` from `RequestContext`.
   - Call `LogEntryMetadata::for_tool_call(server_id, call_id, byte_count,
     &capture_ctx)` to compute the object path and metadata.
   - Call `log_store.store_log(ctx, &metadata, bytes, &retention_policy)`.
   - Record the `object_path` for inclusion in the audit record.
   - Stderr storage is best-effort: a failed write logs a warning but does
     not fail the tool call.
8. Audit: build `ToolCallAuditRecord::from_result(...)` including the
   `stderr_log_path` (the object store path, or `None` if no stderr was
   captured or storage failed). Call `catalog.record_audit()`. Audit is
   best-effort -- a failed audit write logs a warning but does not fail the
   call.
9. Return `ToolCallResult`.

`store_startup_stderr(&self, server_id: McpServerId, stderr: bytes::Bytes) -> ToolDiscoveryRoutingServiceResult<LogEntryMetadata>`:
 stores startup stderr captured from `McpServerHost::start`. Called by the
caller after `lifecycle_service.start()` returns a `LifecycleStartResult` with
non-empty `startup_stderr`. Builds
`LogEntryMetadata::for_startup(server_id, byte_count, &LogCaptureContext)`,
stores via `log_store.store_log(&metadata, stderr, &retention_policy)`, and
returns the metadata. This method also triggers `sweep_expired` for the server
to enforce rotation.

`sweep_expired_logs(&self, server_id: McpServerId) -> ToolDiscoveryRoutingServiceResult<usize>`:
 triggers a retention sweep for a specific server. Delegates to
`log_store.sweep_expired(server_id, &SweepContext { … })`. Returns the count of
deleted log entries.

Define `ToolDiscoveryRoutingServiceError` with variants:
`Domain(from ToolRegistryDomainError)`, `Catalog(from ToolCatalogError)`,
`Registry(from McpServerRegistryError)`, `Host(from McpServerHostError)`,
`Policy(from ToolPolicyError)`, `LogStore(from ToolLogStoreError)`,
`NotFound(McpServerId)`.

In `src/tool_registry/services/discovery/tests.rs`, write unit tests using
`InMemoryToolCatalog`, `InMemoryMcpServerRegistry`, `InMemoryMcpServerHost`,
`AllowAllPolicy`, `ObjectStoreLogAdapter::in_memory()`, and `DefaultClock`:

- `discover_tools_persists_catalog`: register + start server with tool
  catalog, call `discover_and_persist_tools`, verify catalog has entries.
- `discover_tools_requires_running_server`: call on non-running server,
  expect domain error.
- `mark_unavailable_updates_catalog`: discover, then mark unavailable,
  verify entries have `available: false`.
- `call_tool_routes_to_correct_server`: discover tools, configure call
  result, call tool, verify result and audit record.
- `call_tool_unknown_tool_returns_not_found`: call with tool name not in
  catalog.
- `call_tool_unavailable_tool_returns_error`: mark tools unavailable, then
  call.
- `call_tool_schema_validation_failure`: call with parameters missing a
  required field.
- `call_tool_policy_denied`: use `DenyAllPolicy`, verify `PolicyDenied`.
- `call_tool_host_failure_still_records_audit`: configure host to fail,
  verify audit record exists with failure outcome.
- `call_tool_captures_stderr_in_log_store`: configure host with stderr
  output, call tool, verify stderr blob stored in in-memory object store and
  audit record contains the `stderr_log_path`.
- `call_tool_without_stderr_has_no_log_path`: configure host with no stderr,
  verify audit record `stderr_log_path` is `None`.
- `store_startup_stderr_captures_and_sweeps`: start server with startup
  stderr, call `store_startup_stderr`, verify blob stored, then add enough logs
  to exceed `max_logs_per_server` and verify sweep deletes oldest.
- `sweep_expired_logs_deletes_old_entries`: create log entries with
  `expires_at` in the past, call `sweep_expired_logs`, verify deletion.
- `stderr_truncation_at_max_bytes`: configure host with stderr exceeding
  `max_bytes_per_log`, verify stored blob is truncated with marker.
- `call_tool_timeout`: (if feasible with in-memory host; may need a custom
  host adapter that sleeps).

In `src/tool_registry/services/mod.rs`, add `pub mod discovery;` and re-export
`ToolDiscoveryRoutingService` and its error/result types.

Also add domain unit tests:

In `src/tool_registry/domain/validation.rs` (inline `#[cfg(test)] mod tests`):
test `validate_parameters` for valid params, missing required field, non-object
params when object expected, empty required array, and extra fields.

In `src/tool_registry/domain/catalog.rs` (inline `#[cfg(test)] mod tests` or
separate `tests.rs`): test `CatalogEntry` creation, `mark_available`,
`mark_unavailable`, `update_tool`.

In `src/tool_registry/domain/routing.rs` (inline tests): test
`ToolCallRequest::new`, `ToolCallOutcome::is_success`/`is_failure`.

Go/no-go checkpoint: run `make check-fmt`, `make lint`, and `make test`. All
must pass with new tests included.

### Stage D: Integration and BDD tests

In `tests/in_memory/tool_discovery_routing_tests.rs`, write integration tests
exercising both the lifecycle service and discovery/routing service composed
over in-memory adapters:

- `discover_and_call_tool_end_to_end`: register server, start it, discover
  tools, call a tool, verify result and audit.
- `two_servers_route_correctly`: register two servers with different tools,
  discover both, call each tool, verify routing to correct server.
- `tool_unavailable_after_stop`: start, discover, stop, mark unavailable,
  verify call fails.
- `rediscovery_after_restart`: start, discover, stop, mark unavailable,
  restart, rediscover, mark available, verify call succeeds.
- `audit_trail_accumulates`: multiple calls, verify audit count.
- `stderr_captured_for_startup_and_tool_calls`: start server with startup
  stderr, call tool with stderr, verify both log blobs stored in object store
  and references in audit trail.
- `log_rotation_enforces_max_count`: generate more logs than
  `max_logs_per_server`, trigger sweep, verify oldest deleted.
- `log_retention_enforces_expiry`: create logs with past expiry, sweep,
  verify deleted.

Register the module in `tests/in_memory.rs` as
`mod tool_discovery_routing_tests;`.

In `tests/postgres/tool_discovery_routing_tests.rs`, write PostgreSQL
integration tests:

- `catalog_round_trip`: discover tools, verify rows in Postgres, stop
  and verify availability updated.
- `audit_log_persisted`: call tool, verify audit log row.
- `catalog_survives_service_reconstruction`: discover tools, construct a
  new service instance from the same pool, verify catalog entries are still
  present (proving persistence).

Register the module in `tests/postgres.rs` as
`mod tool_discovery_routing_tests;`.

In `tests/features/tool_discovery_routing.feature`, write BDD scenarios:

```gherkin
Feature: Tool discovery and routing

  Scenario: Discover tools from a running MCP server
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "read_file" is available on that server
    When the server is registered and started
    And tools are discovered
    Then the tool catalog contains 1 entry
    And tool "read_file" is marked as available

  Scenario: Route a tool call to the correct server
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "read_file" is available on that server
    And calling tool "read_file" on that server returns '{"content": "hello"}'
    When the server is registered and started
    And tools are discovered
    And tool "read_file" is called with parameters '{"path": "/tmp/test.txt"}'
    Then the tool call succeeds
    And the audit log contains 1 entry for tool "read_file"

  Scenario: Tool becomes unavailable when server stops
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "read_file" is available on that server
    When the server is registered and started
    And tools are discovered
    And the server is stopped
    And tools are marked unavailable
    Then calling tool "read_file" is rejected as unavailable

  Scenario: Unknown tool call is rejected
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    When the server is registered and started
    And tools are discovered
    Then calling tool "nonexistent_tool" is rejected as not found

  Scenario: Tool call stderr is captured in the log store
    Given a stdio MCP server named "workspace_tools" with command "mcp-server"
    And tool "read_file" is available on that server
    And calling tool "read_file" on that server returns '{"content": "hello"}'
    And calling tool "read_file" on that server produces stderr
      'debug: opening file'
    When the server is registered and started
    And tools are discovered
    And tool "read_file" is called with parameters '{"path": "/tmp/test.txt"}'
    Then the tool call succeeds
    And the audit log entry for tool "read_file" has a stderr log path
    And the stored stderr log contains 'debug: opening file'
```

In `tests/tool_discovery_routing_steps.rs`, implement step definitions
following the pattern established in `tests/mcp_server_lifecycle_steps.rs`: a
`ToolDiscoveryWorld` struct holding shared state, `run_async` helper,
`#[given]`/`#[when]`/`#[then]` step functions, and `#[scenario]` bindings.

Go/no-go checkpoint: run `make all`. Must pass with all new and existing tests.

### Stage E: Documentation, roadmap update, and final gates

Update `docs/corbusier-design.md` by appending a dated "Implementation
decisions" subsection under the F-005-RQ-002 and F-005-RQ-003 context,
documenting:

- The `ToolDiscoveryRoutingService` as a sibling service pattern.
- Lightweight schema validation approach (manual, not crate-backed).
- `AllowAllPolicy` default with `ToolPolicyEnforcer` extensibility point.
- Unique tool name constraint with `AmbiguousToolName` error.
- Composition-based lifecycle-to-discovery integration.
- Audit trail via `ToolCallAuditRecord` and `tool_call_audit_log` table.
- Stderr log capture via `ToolLogStore` port backed by `object_store` crate.
- Log retention policy: 7-day default, 10 MiB per-log cap, 100 logs per
  server max, with sweep-on-start rotation.
- `tool_log_metadata` table for log index and expiry tracking.
- Object store path convention:
  `tool_logs/{server_id}/{kind}/{log_entry_id}.stderr`.

Update `docs/users-guide.md` with a new section covering:

- Discovering tools after starting an MCP server.
- Querying the tool catalog.
- Calling a tool by name (routing, validation, policy).
- Viewing the audit trail.
- Tool availability lifecycle (available when server running, unavailable
  when stopped).
- Stderr log capture: how startup and tool call stderr is captured, where
  logs are stored, how to retrieve a log by its path from the audit trail, and
  how to configure retention policy parameters.

Include Rust code examples following the existing style (using in-memory
adapters).

Mark `docs/roadmap.md` item 2.1.2 and its three sub-bullets as done (`[x]`).

Run full quality gates:

```bash
set -o pipefail && make check-fmt 2>&1 | tee /tmp/2-1-2-check-fmt.log
set -o pipefail && make lint 2>&1 | tee /tmp/2-1-2-lint.log
set -o pipefail && make test 2>&1 | tee /tmp/2-1-2-test.log
set -o pipefail && make fmt 2>&1 | tee /tmp/2-1-2-fmt.log
set -o pipefail && PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 \
  | tee /tmp/2-1-2-markdownlint.log
set -o pipefail && make nixie 2>&1 | tee /tmp/2-1-2-nixie.log
```

Go/no-go: all six commands must exit 0. If any fail, fix and re-run.

## Concrete steps

Run all commands from repository root: `/home/user/project`.

1. Stage A compilation checks:

   ```bash
   set -o pipefail && make check-fmt 2>&1 \
     | tee /tmp/2-1-2-stage-a-check-fmt.log
   set -o pipefail && make lint 2>&1 | tee /tmp/2-1-2-stage-a-lint.log
   ```

2. Stage B compilation checks:

   ```bash
   set -o pipefail && make check-fmt 2>&1 \
     | tee /tmp/2-1-2-stage-b-check-fmt.log
   set -o pipefail && make lint 2>&1 | tee /tmp/2-1-2-stage-b-lint.log
   set -o pipefail && make test 2>&1 | tee /tmp/2-1-2-stage-b-test.log
   ```

3. Fast iteration on targeted tests during Stage C:

   ```bash
   set -o pipefail && cargo nextest run --all-targets --all-features \
     tool_discovery 2>&1 | tee /tmp/2-1-2-targeted-tests.log
   set -o pipefail && cargo nextest run --all-targets --all-features \
     validation 2>&1 | tee /tmp/2-1-2-validation-tests.log
   ```

4. Full test suite after Stage C:

   ```bash
   set -o pipefail && make all 2>&1 \
     | tee /tmp/2-1-2-stage-c-make-all.log
   ```

5. Full test suite after Stage D:

   ```bash
   set -o pipefail && make all 2>&1 \
     | tee /tmp/2-1-2-stage-d-make-all.log
   ```

6. Final gates after Stage E:

   ```bash
   set -o pipefail && make all 2>&1 \
     | tee /tmp/2-1-2-final-make-all.log
   set -o pipefail && make fmt 2>&1 | tee /tmp/2-1-2-final-fmt.log
   set -o pipefail && PATH=/root/.bun/bin:$PATH make markdownlint 2>&1 \
     | tee /tmp/2-1-2-final-markdownlint.log
   set -o pipefail && make nixie 2>&1 \
     | tee /tmp/2-1-2-final-nixie.log
   ```

## Validation and acceptance

Acceptance is behavioural:

1. Tools discovered from a running MCP server are persisted in the catalog
   and survive service reconstruction (verified by PostgreSQL integration test
   `catalog_survives_service_reconstruction`).
2. Calling a tool by name routes the request to the correct MCP server and
   returns the result (verified by BDD scenario "Route a tool call to the
   correct server" and integration test `two_servers_route_correctly`).
3. Tool call parameters are validated against the tool's input schema before
   execution (verified by unit test `call_tool_schema_validation_failure`).
4. A pluggable policy enforcement point checks each tool call before
   execution (verified by unit test `call_tool_policy_denied`).
5. Every tool call (success or failure) produces an audit trail record
   (verified by integration tests and BDD scenario).
6. Stopping a server marks its tools as unavailable in the catalog, and
   subsequent tool calls for those tools are rejected (verified by BDD scenario
   "Tool becomes unavailable when server stops").
7. Calling a tool that does not exist returns a typed error (verified by
   BDD scenario "Unknown tool call is rejected").
8. Stderr output from tool calls is captured and stored in the object store,
   with a reference path recorded in the audit trail (verified by BDD scenario
   "Tool call stderr is captured in the log store" and unit test
   `call_tool_captures_stderr_in_log_store`).
9. Startup stderr from MCP server start is captured and stored (verified by
   unit test `store_startup_stderr_captures_and_sweeps`).
10. Log rotation enforces `max_logs_per_server` and `retention_period`
   (verified by unit tests `sweep_expired_logs_deletes_old_entries` and
   integration test `log_rotation_enforces_max_count`).
11. Stderr exceeding `max_bytes_per_log` is truncated with a marker
   (verified by unit test `stderr_truncation_at_max_bytes`).

Quality criteria:

- Tests: all existing tests and new 2.1.2 tests pass (`make test`).
- Lint/format: `make check-fmt` and `make lint` pass.
- Docs validation: `make markdownlint` and `make nixie` pass.
- Roadmap: 2.1.2 and sub-items marked `[x]` in `docs/roadmap.md`.

## Idempotence and recovery

- Test and verification commands are safe to rerun.
- Migration is additive (new tables only, no column changes to existing
  tables); if migration/testing fails, recreate temporary databases via the
  existing PostgreSQL test harness.
- Tool discovery is idempotent: running `discover_and_persist_tools` twice
  for the same server replaces existing catalog entries via `sync_server_tools`
  upsert semantics.
- `mark_tools_unavailable` / `mark_tools_available` are idempotent: calling
  when already in the target state is a no-op.

## Interfaces and dependencies

New external crate dependencies (authorized):

- `object_store = "0.12.0"` -- unified object storage API with
  `LocalFileSystem` and `InMemory` backends. Part of the Apache Arrow
  ecosystem. Used for stderr log blob storage behind the `ToolLogStore` port.
- `bytes = "1.10.1"` -- byte buffer type used in the `ToolLogStore` and
  `McpServerHost` port APIs. Already a transitive dependency via
  `object_store`, `diesel`, and `tokio`, but declared explicitly since it
  appears in public port signatures.

All other implementations use existing workspace crates: `serde_json`,
`chrono`, `uuid`, `async-trait`, `thiserror`, `diesel`, `mockable`, `tokio`.

### New port traits

In `src/tool_registry/ports/catalog.rs`:

```rust
#[async_trait]
pub trait ToolCatalogRepository: Send + Sync {
    async fn sync_server_tools(
        &self,
        server_id: McpServerId,
        entries: &[CatalogEntry],
    ) -> ToolCatalogResult<()>;

    async fn mark_server_tools_unavailable(
        &self,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()>;

    async fn mark_server_tools_available(
        &self,
        server_id: McpServerId,
    ) -> ToolCatalogResult<()>;

    async fn find_by_tool_name(
        &self,
        tool_name: &str,
    ) -> ToolCatalogResult<Option<CatalogEntry>>;

    async fn list_all(&self) -> ToolCatalogResult<Vec<CatalogEntry>>;

    async fn record_audit(
        &self,
        record: &ToolCallAuditRecord,
    ) -> ToolCatalogResult<()>;
}
```

In `src/tool_registry/ports/policy.rs`:

```rust
#[async_trait]
pub trait ToolPolicyEnforcer: Send + Sync {
    async fn evaluate(
        &self,
        tool_name: &str,
        parameters: &Value,
    ) -> Result<PolicyDecision, ToolPolicyError>;
}
```

In `src/tool_registry/ports/log_store.rs`:

```rust
#[async_trait]
pub trait ToolLogStore: Send + Sync {
    async fn store_log(
        &self,
        metadata: &LogEntryMetadata,
        content: bytes::Bytes,
        retention: &LogRetentionPolicy,
    ) -> ToolLogStoreResult<()>;

    async fn retrieve_log(
        &self,
        path: &str,
    ) -> ToolLogStoreResult<bytes::Bytes>;

    async fn delete_log(
        &self,
        path: &str,
    ) -> ToolLogStoreResult<()>;

    async fn list_logs_for_server(
        &self,
        server_id: McpServerId,
    ) -> ToolLogStoreResult<Vec<String>>;

    async fn sweep_expired(
        &self,
        server_id: McpServerId,
        ctx: &SweepContext<'_>,
    ) -> ToolLogStoreResult<usize>;
}
```

### Extended existing trait

In `src/tool_registry/ports/host.rs`, added methods and result types:

```rust
/// Result of starting an MCP server, including captured stderr.
pub struct StartHostResult {
    pub stderr_output: Option<bytes::Bytes>,
}

/// Result of a tool call, including content and captured stderr.
pub struct ToolCallHostResult {
    pub content: Value,
    pub stderr_output: Option<bytes::Bytes>,
}

// Added to McpServerHost trait:
async fn call_tool(
    &self,
    ctx: &RequestContext,
    server: &McpServerRegistration,
    request: &ToolCallRequest,
) -> McpServerHostResult<ToolCallHostResult>;
```

Note: the existing `start` method signature changes from returning
`McpServerHostResult<()>` to `McpServerHostResult<StartHostResult>`. This is a
breaking change to the trait, but `InMemoryMcpServerHost` is the only
implementor and is updated in the same change.

### New service

In `src/tool_registry/services/discovery/mod.rs`:

```rust
pub struct ToolDiscoveryRoutingService<Cat, Reg, H, Pol, Log, C>
where
    Cat: ToolCatalogRepository,
    Reg: McpServerRegistryRepository,
    H: McpServerHost,
    Pol: ToolPolicyEnforcer,
    Log: ToolLogStore,
    C: Clock + Send + Sync,
{
    catalog: Arc<Cat>,
    registry: Arc<Reg>,
    host: Arc<H>,
    policy: Arc<Pol>,
    log_store: Arc<Log>,
    retention_policy: LogRetentionPolicy,
    clock: Arc<C>,
}
```

### File manifest

New files (estimated 23-26):

| Path                                                             | Purpose                                                                |
| ---------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `src/tool_registry/domain/catalog.rs`                            | `CatalogEntry`, `CatalogEntryId`                                       |
| `src/tool_registry/domain/routing.rs`                            | `ToolCallRequest`, `ToolCallResult`, `ToolCallOutcome`, `ToolCallId`   |
| `src/tool_registry/domain/audit.rs`                              | `ToolCallAuditRecord` (with `stderr_log_path`)                         |
| `src/tool_registry/domain/policy.rs`                             | `PolicyDecision`                                                       |
| `src/tool_registry/domain/validation.rs`                         | `validate_parameters()`                                                |
| `src/tool_registry/domain/log_capture.rs`                        | `LogEntryId`, `LogEntryKind`, `LogEntryMetadata`, `LogRetentionPolicy` |
| `src/tool_registry/ports/catalog.rs`                             | `ToolCatalogRepository` port                                           |
| `src/tool_registry/ports/policy.rs`                              | `ToolPolicyEnforcer` port                                              |
| `src/tool_registry/ports/log_store.rs`                           | `ToolLogStore` port                                                    |
| `src/tool_registry/services/discovery/mod.rs`                    | `ToolDiscoveryRoutingService`                                          |
| `src/tool_registry/services/discovery/tests.rs`                  | Service unit tests                                                     |
| `src/tool_registry/adapters/memory/catalog.rs`                   | `InMemoryToolCatalog`                                                  |
| `src/tool_registry/adapters/policy.rs`                           | `AllowAllPolicy`, `DenyAllPolicy`                                      |
| `src/tool_registry/adapters/log_store.rs`                        | `ObjectStoreLogAdapter` (wraps `object_store`)                         |
| `src/tool_registry/adapters/postgres/catalog_schema.rs`          | Diesel table macros (3 tables)                                         |
| `src/tool_registry/adapters/postgres/catalog_models.rs`          | Row models (incl. `LogMetadataRow`)                                    |
| `src/tool_registry/adapters/postgres/catalog_repository.rs`      | `PostgresToolCatalog`                                                  |
| `src/tool_registry/adapters/postgres/log_metadata_repository.rs` | `PostgresLogMetadataRepository`                                        |
| `migrations/2026-03-04-000000_add_tool_catalog_tables/up.sql`    | Migration (3 tables)                                                   |
| `migrations/2026-03-04-000000_add_tool_catalog_tables/down.sql`  | Migration down                                                         |
| `tests/in_memory/tool_discovery_routing_tests.rs`                | In-memory integration                                                  |
| `tests/postgres/tool_discovery_routing_tests.rs`                 | Postgres integration                                                   |
| `tests/features/tool_discovery_routing.feature`                  | BDD scenarios                                                          |
| `tests/tool_discovery_routing_steps.rs`                          | BDD step definitions                                                   |

Modified files (estimated 14-16):

| Path                                          | Changes                                                                          |
| --------------------------------------------- | -------------------------------------------------------------------------------- |
| `Cargo.toml`                                  | Add `object_store` and `bytes` dependencies                                      |
| `src/tool_registry/domain/error.rs`           | Add 6 new error variants                                                         |
| `src/tool_registry/domain/mod.rs`             | Add module declarations and re-exports                                           |
| `src/tool_registry/ports/host.rs`             | Add `call_tool`, change `start` return type, add result structs + error variants |
| `src/tool_registry/ports/mod.rs`              | Add catalog, policy, and log_store re-exports                                    |
| `src/tool_registry/adapters/runtime.rs`       | Add `call_tool` + startup/call stderr to `InMemoryMcpServerHost`                 |
| `src/tool_registry/adapters/memory/mod.rs`    | Export `InMemoryToolCatalog`                                                     |
| `src/tool_registry/adapters/postgres/mod.rs`  | Export `PostgresToolCatalog`, `PostgresLogMetadataRepository`                    |
| `src/tool_registry/adapters/mod.rs`           | Export policy + log_store adapters                                               |
| `src/tool_registry/services/mod.rs`           | Export discovery service                                                         |
| `src/tool_registry/services/lifecycle/mod.rs` | Update `start` call site for new `StartHostResult` return type                   |
| `src/tool_registry/mod.rs`                    | Update module-level doc comment                                                  |
| `tests/in_memory.rs`                          | Add `mod tool_discovery_routing_tests;`                                          |
| `tests/postgres.rs`                           | Add `mod tool_discovery_routing_tests;`                                          |
| `tests/postgres/helpers.rs`                   | Add migration constant and apply call                                            |
| `docs/corbusier-design.md`                    | Add implementation decisions                                                     |
| `docs/users-guide.md`                         | Add tool discovery/routing + log capture section                                 |
| `docs/roadmap.md`                             | Mark 2.1.2 items as done                                                         |

## Artifacts and notes

(To be populated with validation evidence during implementation.)

## Revision note

- 2026-03-04: Initial draft created from roadmap 2.1.2, design sections
  2.2.4, 6.1.4, 4.4.2.2, and 6.4.2.4, and current repository
  testing/architecture conventions. Based on completed 2.1.1 foundation.

- 2026-03-04: Added stderr log capture requirements per user request.
  Integrated `object_store` crate (authorized new dependency) for log blob
  storage. Added `ToolLogStore` port, `ObjectStoreLogAdapter` adapter,
  `LogEntryMetadata` and `LogRetentionPolicy` domain types, `tool_log_metadata`
  table, `stderr_log_path` field on audit records, startup stderr capture on
  `McpServerHost::start`, and log rotation/ retention strategy (7-day default,
  10 MiB cap, 100-log-per-server max). Updated host port return types
  (`StartHostResult`, `ToolCallHostResult`) to carry optional stderr bytes.
  Added 6 new unit tests and 3 new integration tests for log capture, 1 new BDD
  scenario. Updated all affected stages (A-E), file manifest, and validation
  criteria.
