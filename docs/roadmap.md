# Development roadmap

The roadmap translates the Corbusier design into phased, measurable delivery
steps. Work is ordered by dependency and avoids time-based commitments, while
staying within the in-scope capabilities defined in corbusier-design.md.

## 1. Podbot conformance and migration

This phase captures the ADR-driven migration work needed to align Corbusier
with Podbot-hosted execution while renumbering the subsequent roadmap and
execplan references to keep the sequence contiguous. External Podbot
dependencies refer to the
[Podbot development roadmap](https://raw.githubusercontent.com/leynos/podbot/refs/heads/main/docs/podbot-roadmap.md).
 It sits ahead of phases 2 through 7: phase 1 establishes the runtime,
workspace, wire, validation, and security boundaries that the later
orchestration, API, and operator-facing phases assume.

Table 1.0.0: Phase 1 subphases and delivery goals.

| Subphase | Goal                                                                                          |
| -------- | --------------------------------------------------------------------------------------------- |
| 1.1      | Ratify the migration boundary and move hosted execution behind a Podbot-facing seam.          |
| 1.2      | Introduce the canonical workspace runtime model and source-policy controls.                   |
| 1.3      | Shift hosted wire attachment and hook execution onto Podbot control-plane contracts.          |
| 1.4      | Persist durable hosted runtime entities, audit links, retention rules, and conformance gates. |
| 1.5      | Define prompt and bundle artefacts, validation semantics, and least-privilege defaults.       |
| 1.6      | Run staged migration gates and declare the evidence needed to retire legacy paths.            |

Use the shared dependency labels below to keep phase 1 task text readable:

- Phase 1 hosting schema dependency: Podbot Step 1.4, "Hosting schema
  migration and compatibility matrix".
- Phase 1 workspace strategy dependency: Podbot Step 4.4, "Workspace
  strategies".
- Phase 1 hosting core dependencies: Podbot Step 4.5, "Normalized launch
  contract", Podbot Step 4.6, "Hosted session control plane", and Podbot Step
  4.7, "MCP wire provisioning and injection".
- Phase 1 prompt-validation dependencies: Podbot Step 2.6, "ACP capability
  masking enforcement", and Podbot Step 4.8, "Prompt, bundle, and validation
  surfaces".
- Phase 1 hook-recovery dependencies: Podbot Step 4.9, "Hook execution and
  orchestrator acknowledgement", and Podbot Step 4.10, "Recovery, replay, and
  restart safety".
- Phase 1 conformance dependencies: Podbot Step 4.11, "Gated end-to-end (E2E)
  orchestration suite", Podbot Step 8.2, "ACP transport conformance harness",
  Podbot Step 8.3, "Host lifecycle and output-purity tests", and Podbot Step
  8.4, "Wire, hook, and validation conformance tests".

### 1.1. Migration governance and runtime boundary

- [ ] 1.1.1 Ratify the staged migration boundary and phase gates. Requires
  the phase 1 hosting schema dependency and Podbot Step 4.5, "Normalized launch
  contract". See adr-010-migration-and-coexistence-strategy.md §Decision
  Outcome / Proposed Direction and §Migration Plan.
  - [ ] Ratify ADRs 001 through 005 together, so no foundational migration
    ADR carries contradictory dependency or ownership text.
  - [ ] Record advancement criteria for warn-only, compatibility, and blocking
    phases in repository-facing documentation and review checklists.
  - Design note: migration gates should advance only when Podbot-facing
    surfaces are stable enough to avoid reviving inline runtime ownership in
    Corbusier.
  - Outstanding decisions: which phases require accepted ADRs before merge,
    and which roadmap or documentation updates gate the end of migration.
  - [ ] Success criteria: ADRs 001 through 005 cite consistent Podbot
    dependencies, and each migration phase has explicit entry and exit gates.
- [ ] 1.1.2 Establish the Podbot-hosted runtime seam. Requires 2.5.1, Podbot
  Step 4.5, "Normalized launch contract", and Podbot Step 4.6, "Hosted session
  control plane". See adr-001-runtime-boundary-between-corbusier-and-podbot.md
  §Decision Outcome / Proposed Direction and §Migration Plan.
  - [ ] Define a Corbusier port for the Podbot library API and route hosted
    sessions through that port instead of inline runtime ownership.
  - [ ] Define typed adapter errors, retries, and fake-runtime seams that
    mirror the Podbot boundary rather than recreating a second production
    runtime inside Corbusier.
  - Design note: Podbot owns workspace shaping, container lifecycle, MCP wire
    bridging, and generic hook execution; Corbusier owns policy, registry,
    orchestration, durable state, and audit interpretation.
  - Outstanding decisions: whether CLI fallback remains valid for development
    or recovery, which side owns low-level runtime retries, and whether any
    runtime action may bypass Podbot.
  - [ ] Success criteria: all hosted-session launches use the Podbot-facing
    adapter, and no production path depends on inline hosted runtime code.

### 1.2. Workspace runtime model and source policy

- [ ] 1.2.1 Introduce the canonical workspace runtime record. Requires 2.5.2,
  the phase 1 hosting schema dependency, the phase 1 workspace strategy
  dependency, and Podbot Step 4.5, "Normalized launch contract". See
  adr-002-workspace-runtime-model-and-source-policy.md §Decision Outcome /
  Proposed Direction and adr-006-durable-runtime-state-and-audit-model.md
  §Migration Plan.
  - [ ] Persist stable workspace identifiers linked to tenant, task, and
    hosted-session context with explicit lifecycle states.
  - [ ] Record requested source type, access mode, and runtime identifiers
    returned by Podbot for prepared workspaces.
  - Design note: Corbusier owns the logical workspace record; Podbot owns the
    prepared runtime workspace for hosted execution.
  - Outstanding decisions: whether repository clone and host mount share the
    same lifecycle semantics, and how long failed workspaces remain available
    for debugging before cleanup.
  - [ ] Success criteria: workspace records survive restart, link cleanly to
    hosted sessions, and distinguish logical identity from concrete runtime
    state.
- [ ] 1.2.2 Replace legacy transport labels with the canonical MCP source
  taxonomy. Requires 1.2.1, the phase 1 hosting schema dependency, and Podbot
  Step 4.7, "MCP wire provisioning and injection". See
  adr-004-canonical-mcp-source-taxonomy-and-legacy-transport-migration.md
  §Decision Outcome / Proposed Direction and §Migration Plan.
  - [ ] Define canonical source variants for local stdio, helper-container
    stdio, and direct Streamable HTTP sources in Corbusier's domain model.
  - [ ] Add compatibility parsing for legacy transport records and write
    canonical values on update paths.
  - Design note: agent-visible wire endpoints are runtime artefacts and must
    not be persisted as source definitions.
  - Outstanding decisions: how much automatic migration is safe for ambiguous
    legacy records, and which health semantics belong to sources versus wires.
  - [ ] Success criteria: new writes use canonical taxonomy values only, while
    compatibility reads still accept retained legacy transport records.
- [ ] 1.2.3 Enforce workspace source safety and access policy. Requires 1.2.1,
  Podbot Step 2.2, "Container creation", Podbot Step 2.3, "Credential
  injection", Podbot Step 3.1, "App authentication", Podbot Step 3.2,
  "Installation token acquisition", Podbot Step 3.3, "Token daemon", Podbot
  Step 3.4, "GIT_ASKPASS mechanism (Git credential helper variable)", and the
  phase 1 workspace strategy dependency. See
  adr-002-workspace-runtime-model-and-source-policy.md §Decision Outcome /
  Proposed Direction and adr-009-security-and-privilege-boundary-defaults.md
  §Decision Outcome / Proposed Direction.
  - [ ] Add canonical path resolution, allowed-root validation, and symlink
    escape protection for host mounts before Podbot is allowed to mount them.
  - [ ] Enforce explicit read-only or read-write access modes for clone-backed
    and host-mounted workspaces, including helper-container repository access.
  - Design note: the initial production default is repository clone into a
    Podbot-owned runtime workspace, with host mounts allowed only through
    explicit policy and safety checks.
  - Outstanding decisions: whether hooks may widen access from read-only to
    read-write, and whether helper-container sources may inherit repository
    access by default.
  - [ ] Success criteria: unsafe mount paths are rejected deterministically,
    Git-backed workspace preparation uses rotated credentials, and access mode
    is recorded for every prepared workspace.

### 1.3. Hosted tool plane and hook control

- [ ] 1.3.1 Move hosted MCP attachment onto Podbot-owned wire provisioning.
  Requires 1.1.2, 1.2.2, Podbot Step 2.5, "Protocol-safe execution (stdio
  proxy)", and the phase 1 hosting core dependencies. See
  adr-003-mcp-wire-model-and-tool-plane-ownership.md §Decision Outcome /
  Proposed Direction and §Migration Plan.
  - [ ] Separate Corbusier's source catalogue from runtime wire attachment and
    request Podbot-managed wire sets for each hosted workspace.
  - [ ] Publish the agent-visible wire view at session start and ingest
    wire-related lifecycle events for audit and operator visibility.
  - Design note: Corbusier remains the tool catalogue and policy authority,
    but the hosted agent becomes the MCP client for Podbot-hosted sessions.
  - Outstanding decisions: how tool-call telemetry enters Corbusier when it is
    no longer inline, whether mixed-mode tool invocation is ever justified,
    and whether wire attachment may change during a live session.
  - [ ] Success criteria: hosted agents receive workspace-scoped wire
    attachments from Podbot, and stdout remains free of Corbusier-owned
    protocol noise.
- [ ] 1.3.2 Replace inline hook assumptions with control-channel
  acknowledgement flow. Requires 1.1.2, Podbot Step 4.6, "Hosted session
  control plane", and the phase 1 hook-recovery dependencies. See
  adr-005-hook-execution-contract-and-control-channel-semantics.md §Decision
  Outcome / Proposed Direction and §Migration Plan.
  - [ ] Define typed hook request, acknowledgement, completion, timeout, and
    abort message shapes with stable correlation identifiers.
  - [ ] Suspend the hosted execution path until Corbusier replies with an
    idempotent approval, denial, or abort decision.
  - Design note: Podbot executes hooks after acknowledgement, while Corbusier
    evaluates policy and records audit outcomes over a dedicated control
    channel.
  - Outstanding decisions: whether denial aborts the session, skips the hook,
    or fails the current step, and whether any completion event may be optional
    for specific trigger types.
  - [ ] Success criteria: hosted hook flows are restart-safe, duplicate
    deliveries are idempotent, and every completed hook has a matching
    acknowledgement record.
- [ ] 1.3.3 Align hosted-session launch and command surfaces with the Podbot
  control plane. Requires 1.3.1, Podbot Step 4.5, "Normalized launch contract",
  Podbot Step 4.6, "Hosted session control plane", Podbot Step 6.1, "Subcommand
  dispatch", and Podbot Step 6.5, "Host subcommand". See
  adr-001-runtime-boundary-between-corbusier-and-podbot.md §Migration Plan and
  adr-010-migration-and-coexistence-strategy.md §Podbot roadmap dependencies.
  - [ ] Route hosted launch requests through the same normalized command and
    library contract used by Podbot's `host` interface.
  - [ ] Ensure Corbusier-facing library and operator tooling consume one typed
    control surface instead of CLI scraping or ad hoc wrapper logic.
  - Design note: Corbusier should integrate against Podbot's normative library
    and command contracts, not against transient CLI text.
  - Outstanding decisions: whether any low-level development tooling may
    remain CLI-bound after the hosted control plane is stable.
  - [ ] Success criteria: hosted control operations use one normalized launch
    contract across library, operator, and recovery paths.

### 1.4. Durable runtime state and audit ingestion

- [ ] 1.4.1 Persist hosted runtime entities with idempotent state machines.
  Requires 1.2.1, 1.3.1, 1.3.2, Podbot Step 4.6, "Hosted session control
  plane", and the phase 1 hook-recovery dependencies. See
  adr-006-durable-runtime-state-and-audit-model.md §Decision Outcome / Proposed
  Direction and §Migration Plan.
  - [ ] Persist hosted sessions, workspaces, wires, hook invocations,
    acknowledgements, and validation snapshots with explicit terminal and
    non-terminal states.
  - [ ] Reconcile restart and replay events idempotently using stable
    correlation identifiers and event identifiers from Podbot.
  - Design note: large logs, transcripts, and runtime artefacts belong outside
    the primary runtime tables and should be linked by reference.
  - Outstanding decisions: whether direct tool-call telemetry arrives as raw
    events or derived audit logs, and which failure states are terminal versus
    reconcilable.
  - [ ] Success criteria: restart recovery replays do not duplicate runtime
    state transitions, and every persisted runtime record links to a tenant,
    task, and hosted session.
- [ ] 1.4.2 Add retention, cleanup, and conformance gates for runtime state.
  Requires 1.4.1 and the phase 1 conformance dependencies. See
  adr-006-durable-runtime-state-and-audit-model.md §Migration Plan and
  adr-010-migration-and-coexistence-strategy.md §Migration Plan.
  - [ ] Move cleanup and retention jobs onto the hosted-session, workspace,
    wire, and hook entities introduced in 1.4.1.
  - [ ] Gate phase advancement on end-to-end orchestration, transport
    conformance, host lifecycle, and wire or hook validation suites.
  - Design note: phase advancement should depend on conformance evidence rather
    than on successful ad hoc manual runs.
  - Outstanding decisions: whether retention differs by tenant or runtime
    surface, and how cleanup avoids deleting data still needed for review or
    incident analysis.
  - [ ] Success criteria: cleanup preserves immutable audit references, and
    hosted conformance suites pass before downstream runtime-facing phases
    advance.

### 1.5. Prompt, bundle, validation, and privilege defaults

- [ ] 1.5.1 Define the prompt, skill, and bundle document model. Requires
  1.4.1, Podbot Step 4.5, "Normalized launch contract", Podbot Step 4.8,
  "Prompt, bundle, and validation surfaces", and Podbot Step 5.3, "Stabilize
  public library boundaries". See
  adr-007-prompt-skill-and-bundle-document-model.md §Decision Outcome /
  Proposed Direction and §Migration Plan.
  - [ ] Define prompt document frontmatter, bundle manifest fields, namespaced
    extension fields, and attachment references.
  - [ ] Add repository parsing, rendering, and bundle assembly logic that keeps
    standard skill directories portable.
  - Design note: bundles sit above standard skill directories and group
    prompts, skill selections, wire defaults, hook defaults, and other runtime
    dependencies.
  - Outstanding decisions: whether frontmatter is templated by default,
    whether bundles may carry runtime defaults, and how artefact versioning and
    immutability are represented.
  - [ ] Success criteria: prompt and bundle artefacts render deterministically
    from repository state, and documentation examples match the normative
    schema.
- [ ] 1.5.2 Implement structured prompt validation with capability
  dispositions. Requires 1.5.1, the phase 1 prompt-validation dependencies, and
  Podbot Step 4.5, "Normalized launch contract". See
  adr-008-prompt-validation-semantics-and-capability-dispositions.md §Decision
  Outcome / Proposed Direction and §Migration Plan.
  - [ ] Define typed validation request and response shapes, including the
    `native`, `host-enforced`, `translated`, `ignored`, and `rejected`
    dispositions.
  - [ ] Return structured diagnostics and effective prompt previews for
    supported hosted targets, and persist representative degraded-case
    fixtures.
  - Design note: required capabilities ending in `ignored` or `rejected`
    should block execution, while preferred capabilities may degrade with a
    warning during the migration window.
  - Outstanding decisions: whether validation is exposed through a user-facing
    command as well as the library surface, whether rendering happens before or
    after validation, and whether validation results are persisted for audit.
  - [ ] Success criteria: validation reports deterministic diagnostics for
    supported hosted targets, and fixture coverage includes blocked, degraded,
    and clean outcomes.
- [ ] 1.5.3 Enforce least-privilege defaults and override controls. Requires
  1.2.3, 1.3.2, 1.5.2, the phase 1 hosting schema dependency, the phase 1
  workspace strategy dependency, the phase 1 prompt-validation dependencies,
  Podbot Step 4.7, "MCP wire provisioning and injection", and Podbot Step 4.9,
  "Hook execution and orchestrator acknowledgement". See
  adr-009-security-and-privilege-boundary-defaults.md §Decision Outcome /
  Proposed Direction and §Migration Plan.
  - [ ] Define the default privilege matrix for hosted agents, hooks,
    helper-container sources, and delegated host capabilities.
  - [ ] Persist override records, approval hooks, and audit capture for each
    privileged deviation from the default matrix.
  - Design note: hosted agents begin with the narrowest access needed for the
    task and prompt, and helper-container sources default to no repository
    access unless policy grants otherwise.
  - Outstanding decisions: who may approve privilege overrides, whether
    development-mode overrides remain ephemeral, and which capability
    combinations remain permanently unsupported.
  - [ ] Success criteria: unsafe privilege combinations fail validation or
    policy evaluation by default, and every override request is reviewable and
    auditable.

### 1.6. Migration closure and legacy retirement

- [ ] 1.6.1 Run staged compatibility, warn-only, and blocking migration gates.
  Requires 1.1.1, 1.2.2, 1.3.1, 1.4.2, 1.5.2, 1.5.3, the phase 1 hosting schema
  dependency, the phase 1 hosting core dependencies, the phase 1
  prompt-validation dependencies, and the phase 1 hook-recovery dependencies.
  See adr-010-migration-and-coexistence-strategy.md §Decision Outcome /
  Proposed Direction and §Migration Plan.
  - [ ] Run warn-only validation where blocking behaviour would break active
    flows, and record diagnostics for reviewed prompt and bundle samples.
  - [ ] Freeze legacy routing and legacy transport labels on new writes while
    retaining compatibility reads for historical records until retirement.
  - Design note: migration should move from compatibility, to warning, to
    blocking behaviour with explicit evidence at each gate rather than by
    silent tightening.
  - Outstanding decisions: how long legacy transport parsing remains
    supported, and when in-memory or legacy runtime adapters can be deleted.
  - [ ] Success criteria: compatibility reads still succeed for retained
    history, while new writes and CI gates move onto blocking behaviour in the
    documented order.
- [ ] 1.6.2 Declare end-of-migration retirement criteria. Requires 1.6.1 and
  TBD (non-linear dependency on the final hosted cutover evidence bundle). See
  adr-010-migration-and-coexistence-strategy.md §Migration Plan and
  §Outstanding Decisions.
  - [ ] Define the evidence bundle required to remove inline hosted runtime
    ownership, legacy routing, and legacy transport write paths.
  - [ ] Require roadmap, ADR, and operator-documentation updates before any
    legacy removal pull request is considered complete.
  - Design note: migration is complete only when the architectural boundary,
    operator expectations, and test gates all point at the same hosted path.
  - Outstanding decisions: what granularity of acceptance criteria is required
    for final phase transitions, and which documentation changes must land
    before legacy retirement.
  - [ ] Success criteria: legacy path removals are blocked until the evidence
    bundle, documentation updates, and conformance gates are all complete.

## 2. Core orchestration foundation

### 2.1. Conversation management

- [x] 2.1.1 Implement the canonical message format and validation. See
  corbusier-design.md §2.2.1.
  - [x] Define user, assistant, and tool event schemas. See
    corbusier-design.md §2.2.1.
  - [x] Add versioned schema validation at ingestion boundaries. See
    corbusier-design.md §4.4.2.1.
  - [x] Success criteria: all stored messages conform to the canonical schema.
- [x] 2.1.2 Persist message history with audit trails. See
  corbusier-design.md §2.2.1 and §6.2.3.
  - [x] Implement conversation history persistence with immutable ordering. See
    corbusier-design.md §6.2.3.
  - [x] Store audit metadata for tool calls and agent responses. See
    corbusier-design.md §2.1.1.
  - [x] Success criteria: conversation history is queryable by conversation id
    with complete audit metadata.
- [x] 2.1.3 Preserve context across agent handoffs. See
  corbusier-design.md §2.2.1 and §4.1.1.1.
  - [x] Persist handoff metadata between agent turns. See
    corbusier-design.md §4.2.1.1.
  - [x] Maintain context window snapshots per agent session. See
    corbusier-design.md §2.2.1.
  - [x] Success criteria: every handoff references the prior turn and tool
    calls used to reach the handoff.

### 2.2. Task lifecycle management

- [x] 2.2.1 Implement issue-to-task creation and tracking. See
  corbusier-design.md §2.2.2.
  - [x] Map external issue metadata into internal task records. See
    corbusier-design.md §2.2.2.
  - [x] Generate task identifiers and lifecycle timestamps. See
    corbusier-design.md §4.3.1.2.
  - [x] Success criteria: tasks can be created from issues and retrieved by
    external issue reference.
- [x] 2.2.2 Associate branches and pull requests with tasks. Requires 2.2.1. See
  corbusier-design.md §2.2.2.
  - [x] Persist branch identifiers alongside task records. See
    corbusier-design.md §2.2.2.
  - [x] Map pull request identifiers to task state updates. See
    corbusier-design.md §2.2.2 and §4.1.1.2.
  - [x] Success criteria: task records include branch and pull request
    references for all linked work items.
- [x] 2.2.3 Enforce task state transitions with validation. Requires
  2.2.1.[^1]
  - [x] Define allowed transitions and terminal states.[^2]
  - [x] Reject invalid transitions with typed errors.[^3]
  - [x] Success criteria: unit tests cover all 36 source/target transition
    pairs, invalid transitions return typed error variants, and terminal states
    reject all outgoing transitions in tested paths.

[^1]: [docs/corbusier-design.md](docs/corbusier-design.md) §4.3.1.2 and
  §4.4.1.1.
[^2]: [docs/corbusier-design.md](docs/corbusier-design.md) §4.3.1.2.
[^3]: [docs/corbusier-design.md](docs/corbusier-design.md) §4.4.1.1.

### 2.3. Agent backend orchestration

- [x] 2.3.1 Implement agent backend registration and discovery. See
  corbusier-design.md §2.2.3.
  - [x] Define backend capability metadata and registration flow. See
    corbusier-design.md §2.2.3.
  - [x] Persist backend registry entries in the persistence layer. See
    corbusier-design.md §6.2.3.
  - [x] Success criteria: at least two backends can be registered and listed
    via the registry API.
- [x] 2.3.2 Orchestrate agent turn execution and sessions. Requires 3.3.1 and
  2.1.1. See corbusier-design.md §2.2.3.
  - [x] Coordinate turn execution with tool calls and responses. See
    corbusier-design.md §4.2.1.1.
  - [x] Maintain session state and expiry rules. See
    corbusier-design.md §2.2.3.
  - [x] Success criteria: agent turns execute with consistent tool routing and
    session continuity.
- [ ] 2.3.3 Translate tool schemas per backend. Requires 3.1.1. See
  corbusier-design.md §2.2.3 and §2.2.4.
  - [ ] Implement schema translation mapping for each backend. See
    corbusier-design.md §2.2.3.
  - [ ] Validate translated schemas against MCP tool definitions. See
    corbusier-design.md §2.2.4.
  - [ ] Success criteria: tool schemas are accepted by each registered backend
    without manual edits.

### 2.4. Slash command system

- [x] 2.4.1 Deliver slash command parsing and template execution. Requires
  2.1.1. See corbusier-design.md §2.1.1.
  - [x] Implement command parser and registry. See corbusier-design.md §2.1.1.
  - [x] Add template expansion and parameter validation. See
    corbusier-design.md §2.1.1.
  - [x] Success criteria: commands produce deterministic tool call sequences
    with auditable records.

### 2.5. Tenant context and identity isolation

- [x] 2.5.1 Establish tenant primitives and request context plumbing. See
  corbusier-design.md §2.1.5 and §2.2.5.
  - [x] Add `TenantId`, `TenantSlug`, and `Tenant` domain primitives. See
    corbusier-design.md §2.2.5.
  - [x] Model initial tenancy as one owning user per tenant, while preserving a
    separate user-versus-tenant identity model for future team and
    organization tenants. See corbusier-design.md §2.2.5.
  - [x] Replace message-only audit context usage with cross-cutting
    `RequestContext` carrying tenant, correlation, causation, user, and
    session identifiers.
    See corbusier-design.md §2.2.5.
  - [x] Success criteria: repository/service signatures require tenant-aware
    request context for tenant-owned operations.
- [x] 2.5.2 Deliver tenant-aware schema and constraints. Requires 2.5.1. See
  corbusier-design.md §6.2.1 and §6.2.2.
  - [x] Create `tenants` and add `tenant_id` to tenant-owned tables. See
    corbusier-design.md §2.2.5.
  - [x] Scope task issue-origin uniqueness and backend registration uniqueness
    by tenant. See corbusier-design.md §2.2.2 and §2.2.3.
  - [x] Enforce parent/child tenant consistency with composite foreign keys.
    See corbusier-design.md §6.2.1.
  - [x] Success criteria: schema changes are in place for tenant-owned
    orchestration data, including `tenant_id`, tenant-scoped uniqueness, and
    composite foreign keys; end-to-end two-tenant reuse and isolation proof is
    tracked under 1.5.4 after 1.5.3 delivers full query scoping and RLS.
- [ ] 2.5.3 Enforce tenant boundaries in adapters and PostgreSQL. Requires
  2.5.2. See corbusier-design.md §6.2.2 and §6.2.3. Note: 2.5.1 added
  `&RequestContext` to port signatures; this item makes adapters *use* the
  tenant context for query scoping and Row-Level Security (RLS) enforcement.
  - [ ] Update adapter implementations to extract and apply `tenant_id` from
    `RequestContext` in all queries and mutations. See
    corbusier-design.md §2.2.5.
  - [ ] Set `set_config('app.tenant_id', <value>, true)` inside each
    transaction, and enable RLS policies on tenant-owned tables. See
    corbusier-design.md §6.2.3.5.
  - [ ] Extend audit trigger/session variable capture to include tenant ID. See
    corbusier-design.md §6.2.3.4.
  - [ ] Success criteria: cross-tenant reads/writes are blocked by both Rust
    signatures and PostgreSQL RLS.
- [ ] 2.5.4 Prove multi-tenant isolation with two-tenant scenarios. Requires
  2.5.3. See corbusier-design.md §2.2.5.
  - [ ] Add tests where two tenants share the same external issue identifiers
    without violating constraints. See corbusier-design.md §2.2.5.
  - [ ] Add tests where two tenants register the same backend name without
    collisions. See corbusier-design.md §2.2.5.
  - [ ] Add negative tests showing tenant A context cannot retrieve tenant B
    records. See corbusier-design.md §2.2.5.
  - [ ] Success criteria: isolation tests fail when tenant scoping is missing
    and pass when tenant context and RLS are correctly applied.

## 3. Tool plane and workflow governance

### 3.1. MCP hosting and tool registry

- [x] 3.1.1 Implement MCP server lifecycle management. Requires 2.3.1. See
  corbusier-design.md §2.2.4 and §6.1.4.
  - [x] Support MCP server start, stop, and health reporting. See
    corbusier-design.md §2.2.4.
  - [x] Register MCP servers in the tool registry. See
    corbusier-design.md §2.2.4.
  - [x] Success criteria: MCP servers can be started, listed, and queried for
    available tools.
- [x] 3.1.2 Deliver tool discovery and routing. Requires 3.1.1. See
  corbusier-design.md §2.2.4 and §6.1.4.
  - [x] Implement tool discovery and catalogue persistence. See
    corbusier-design.md §2.2.4.
  - [x] Route tool calls through the registry with policy checks. See
    corbusier-design.md §6.1.4 and §6.4.2.4.
  - [x] Record audit trail entries for tool calls (success, failure, and
    pre-execution rejections) including timing, parameters, and outcome. See
    corbusier-design.md §2.2.4.
  - [x] Capture startup and per-tool-call stderr via `ToolLogStore` port with
    `ObjectStoreLogAdapter` backend. See corbusier-design.md §2.2.4.
  - [x] Enforce stderr log retention policy (7-day default, 10 MiB cap,
    100-log-per-server maximum) with automated sweeps. See
    corbusier-design.md §2.2.4.
  - [x] Success criteria: tool execution requests are routed to the correct MCP
    server with consistent metadata; audit trail records all outcomes; stderr
    blobs are persisted and rotated per retention policy.

### 3.2. Weaver file editing integration

- [ ] 3.2.1 Enforce Weaver as the authoritative file editor. Requires 3.1.1. See
  corbusier-design.md §2.1.2.
  - [ ] Disable direct file edits from agent backends. See
    corbusier-design.md §2.1.2.
  - [ ] Persist ChangeSet metadata from Weaver. See corbusier-design.md §2.1.2.
  - [ ] Success criteria: all file changes are represented as Weaver ChangeSets
    with diff metadata.

### 3.3. Hook engine and policy enforcement

The tasks below establish the current Corbusier-owned hook baseline. Phase 1.3
extends hosted-session execution onto Podbot control-channel acknowledgement
flows without regressing existing policy enforcement.

- [x] 3.3.1 Implement hook engine execution. Requires 3.1.2 and TBD
  (non-linear dependency on future Weaver workflow milestones). See
  corbusier-design.md §2.1.3 and §6.3.3.
  - [x] Define hook triggers for turn start/end, tool use before/after, and
    pre- and post-commit, pre- and post-merge, pre- and post-pull, pre- and
    post-push, and pre- and post-deploy events. See
    corbusier-design.md §2.1.3.
  - [x] Execute hooks with structured outcomes and logs. See
    corbusier-design.md §6.3.3.
  - [x] Success criteria: hook execution results are recorded for every
    configured trigger.
- [x] 3.3.2 Add policy enforcement and audit capture. Requires 3.3.1. See
  corbusier-design.md §2.1.3 and §6.4.2.5.
  - [x] Implement policy evaluation at enforcement points. See
    corbusier-design.md §6.4.2.4.
  - [x] Persist policy violations and audit events. See
    corbusier-design.md §6.4.2.5.
  - [x] Success criteria: policy enforcement outcomes are queryable by task,
    conversation, and hook event.

### 3.4. Encapsulation and workspace management

Phase 1.2 expands this work from generic encapsulation into a canonical
workspace runtime model shared with Podbot-hosted execution and source-policy
enforcement.

- [ ] 3.4.1 Implement workspace encapsulation for tool execution. Requires
  3.1.1. See corbusier-design.md §2.1.3 and §6.2.4.
  - [ ] Provision Podbot-backed workspaces per task. See
    corbusier-design.md §6.2.4.
  - [ ] Enforce workspace isolation for tool and VCS operations. See
    corbusier-design.md §6.4.4.3.
  - [ ] Success criteria: tool execution occurs only within encapsulated
    workspaces with auditable boundaries.

## 4. External integrations and interfaces

### 4.1. VCS integration and review ingestion

- [ ] 4.1.1 Deliver VCS adapter for GitHub and GitLab. Requires 2.2.3, TBD
  (non-linear dependency: 3.2.2), Podbot Step 3.1, "App authentication", Podbot
  Step 3.2, "Installation token acquisition", Podbot Step 3.3, "Token daemon",
  and Podbot Step 3.4, "GIT_ASKPASS mechanism (Git credential helper
  variable)". See corbusier-design.md §2.1.4 and §6.3.1.
  - [ ] Implement branch, pull request, and issue operations. See
    corbusier-design.md §2.1.4.
  - [ ] Map VCS events into task lifecycle updates. See
    corbusier-design.md §4.1.1.2.
  - [ ] Success criteria: tasks remain synchronised with VCS state transitions.
- [ ] 4.1.2 Implement review ingestion workflows. Requires 4.1.1. See
  corbusier-design.md §2.1.4 and §6.3.2.
  - [ ] Ingest review comments and map them to tasks. See
    corbusier-design.md §6.3.2.
  - [ ] Store review metadata for audit and reporting. See
    corbusier-design.md §2.1.3.
  - [ ] Success criteria: review comments are attached to the relevant task and
    conversation records.

### 4.2. HTTP API surface

- [ ] 4.2.1 Expose core HTTP APIs for conversations and tasks. Requires 2.2.3.
  See corbusier-design.md §2.1.4 and §6.2.1.
  - [ ] Implement conversation, task, and tool endpoints. See
    corbusier-design.md §6.2.1.
  - [ ] Apply API versioning and authentication requirements. See
    corbusier-design.md §6.3.1.5 and §6.4.1.
  - [ ] Success criteria: API endpoints return versioned responses and enforce
    authentication.

### 4.3. Real-time event streaming

- [ ] 4.3.1 Provide real-time event streaming for orchestration updates.
  Requires 5.2.1 (non-linear) and 1.4.1. See corbusier-design.md §2.1.4 and
  §6.2.2.
  - [ ] Implement event schema for conversation and task updates. See
    corbusier-design.md §6.2.2.
  - [ ] Publish events through SSE or equivalent transport. See
    corbusier-design.md §6.2.2 and §7.4.2.
  - [ ] Success criteria: subscribers receive ordered event streams with retry
    support.

### 4.4. Operator and developer user interfaces

- [ ] 4.4.1 Deliver the task management interface. Requires 4.2.1. See
  corbusier-design.md §7.2.1 and §7.5.1.
  - [ ] Implement task list, status, and milestone views. See
    corbusier-design.md §7.5.1.
  - [ ] Add task detail panels with audit history. See
    corbusier-design.md §7.2.1.
  - [ ] Success criteria: tasks can be filtered by status, owner, and milestone.
- [ ] 4.4.2 Deliver the conversation management interface. Requires 4.2.1. See
  corbusier-design.md §7.2.2 and §7.5.2.
  - [ ] Render canonical message history with tool call metadata. See
    corbusier-design.md §7.2.2.
  - [ ] Provide agent handoff visibility and annotations. See
    corbusier-design.md §7.5.2.
  - [ ] Success criteria: conversations show complete message history with
    tool call details.

## 5. Reliability, security, and operations

### 5.1. Security and access control

- [ ] 5.1.1 Implement authentication and session management. Requires 4.2.1. See
  corbusier-design.md §6.4.1.
  - [ ] Add identity management and token handling. See
    corbusier-design.md §6.4.1.1 and §6.4.1.4.
  - [ ] Enforce session expiry and rotation policies. See
    corbusier-design.md §6.4.1.3.
  - [ ] Success criteria: authenticated sessions expire and rotate according to
    policy.
- [ ] 5.1.2 Implement RBAC and resource authorisation. Requires 5.1.1. See
  corbusier-design.md §6.4.2.
  - [ ] Define role hierarchy and permissions. See
    corbusier-design.md §6.4.2.1 and §6.4.2.2.
  - [ ] Enforce authorisation checks at policy enforcement points. See
    corbusier-design.md §6.4.2.4.
  - [ ] Success criteria: all API endpoints require explicit authorisation for
    protected resources.

### 5.2. Observability and monitoring

- [ ] 5.2.1 Implement metrics, logging, and tracing pipelines. Requires 3.3.1.
  See corbusier-design.md §6.5.1 and §6.5.4.
  - [ ] Instrument core services with metrics and traces. See
    corbusier-design.md §6.5.1.1 and §6.5.1.3.
  - [ ] Centralise log aggregation with correlation ids. See
    corbusier-design.md §6.5.1.2.
  - [ ] Success criteria: metrics, traces, and logs share a common correlation
    identifier per request.
- [ ] 5.2.2 Deliver monitoring dashboards and alerting. Requires 5.2.1. See
  corbusier-design.md §6.5.1.5 and §8.5.
  - [ ] Create dashboards for task completion, agent utilisation, and tool
    execution. See corbusier-design.md §8.5.1.
  - [ ] Define alert thresholds and routing. See
    corbusier-design.md §6.5.1.4 and §6.5.3.1.
  - [ ] Success criteria: alerting covers latency, error rate, and availability
    thresholds.

### 5.3. Testing and quality gates

- [ ] 5.3.1 Implement automated unit, integration, and end-to-end test suites.
  Requires 3.3.1 and TBD (non-linear dependency on future Weaver workflow test
  harness milestones). See corbusier-design.md §6.6.1.
  - [ ] Add unit tests for domain services and ports. See
    corbusier-design.md §6.6.1.1.
  - [ ] Add integration tests for VCS, tool, and agent adapters. See
    corbusier-design.md §6.6.1.2.
  - [ ] Success criteria: test suites cover critical workflows without manual
    setup.
- [ ] 5.3.2 Enforce CI quality gates for formatting, linting, and test runs.
  Requires 5.3.1. See corbusier-design.md §6.6.2 and §8.4.1.
  - [ ] Configure CI to run formatter, linter, and test pipelines. See
    corbusier-design.md §8.4.1.
  - [ ] Block merges on failed quality gates. See
    corbusier-design.md §6.6.2.1.
  - [ ] Success criteria: no mainline merge occurs without passing quality
    gates.

### 5.4. Deployment and resilience

- [ ] 5.4.1 Deliver containerised deployment and rollback workflows. Requires
  5.3.2. See corbusier-design.md §8.2 and §8.4.3.
  - [ ] Build multi-stage container images with security scanning. See
    corbusier-design.md §8.2.2 and §8.2.5.
  - [ ] Implement deployment and rollback procedures. See
    corbusier-design.md §8.4.2 and §8.4.3.
  - [ ] Success criteria: deployments support automated rollback on failed
    validation.

### 5.5. Performance and scalability

- [ ] 5.5.1 Validate performance, scalability, and SLA targets. Requires 5.2.2.
  See corbusier-design.md §4.5 and §6.2.4.
  - [ ] Execute performance tests against response time targets. See
    corbusier-design.md §4.5.1.1.
  - [ ] Validate horizontal scaling behaviour under concurrency targets. See
    corbusier-design.md §4.5.3.1.
  - [ ] Success criteria: response latency remains under target thresholds and
    scaling tests meet concurrency goals.

## 6. Front-end API surface and data-model extensions

### 6.1. API contracts and scaffolding

- [ ] 6.1.1 Publish versioned OpenAPI specification and central error schema.
  Requires 4.2.1. See corbusier-api-design.md §HTTP API surface, pagination,
  SSE, and error contracts.
  - [ ] Define `ErrorCode` enum and `Error` response schema compatible with
    Wildside. See corbusier-api-design.md §Error and validation contract.
  - [ ] Generate `/api/v1` OpenAPI document covering error, pagination, and
    auth contracts.
  - [ ] Success criteria: error responses are validated against the schema in
    contract tests.
- [ ] 6.1.2 Implement reusable keyset pagination crate. Requires 6.1.1. See
  corbusier-api-design.md §Pagination semantics.
  - [ ] Implement cursor encoding and decoding with opaque tokens. See
    corbusier-api-design.md §Pagination semantics.
  - [ ] Implement `Paginated<T>` envelope with `data`, `limit`, and hypermedia
    `links` (self, next, and prev). See corbusier-api-design.md §Pagination
    semantics.
  - [ ] Success criteria: pagination envelope shape matches TanStack Query
    infinite query expectations; absence of `next` indicates end-of-list.
- [ ] 6.1.3 Add domain event persistence and SSE endpoint skeleton. Requires
  6.1.1. See corbusier-api-design.md §SSE event stream and replay semantics.
  - [ ] Create `domain_events` table (`tenant_id`, `aggregate_id`,
    `aggregate_type`, `event_type`, `event_data`, and `occurred_at`). See
    corbusier-api-design.md §Replay storage.
  - [ ] Implement SSE endpoint skeleton at `GET /api/v1/events` with event
    identifier emission and `Last-Event-ID` parsing. See
    [HTML Standard: Server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html)
    and
    [MDN: Using server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events)
    for `Last-Event-ID` replay semantics.
  - [ ] Success criteria: SSE endpoint emits well-formed events with stable
    identifiers; `Last-Event-ID` header is parsed on reconnect.

### 6.2. Project and task read models

- [ ] 6.2.1 Introduce project aggregate and bootstrap default projects.
  Requires 2.5.1. See corbusier-api-design.md §Project domain.
  - [ ] Implement `ProjectAggregate` with slug, localized name and
    description, lead, date range, status, and team membership. See
    corbusier-api-design.md §Project domain.
  - [ ] Seed a default project per tenant and attach existing tasks. See
    corbusier-api-design.md §Project domain -- Migration.
  - [ ] Success criteria: projects can be created, listed, and retrieved by
    slug with tenant scoping.
- [ ] 6.2.2 Extend task aggregate with front-end fields. Requires 6.2.1 and
  2.2.3. See corbusier-api-design.md §Task domain.
  - [ ] Add localization, priority, labels, assignment, scheduling, and
    hierarchy reference fields to task persistence. See
    corbusier-api-design.md §Task domain -- Proposed write-side model.
  - [ ] Introduce `Planned` state and update the transition matrix. See
    corbusier-api-design.md §Task domain -- Lifecycle and transition rules.
  - [ ] Backfill `localizations["en-GB"].name` from existing
    `TaskOrigin::Issue` snapshot titles. See corbusier-api-design.md §Task
    domain -- Migration strategy from current models.
  - [ ] Success criteria: all task state transition pairs (including `Planned`)
    are unit-tested; existing tasks are retrievable with localized names.
- [ ] 6.2.3 Deliver task and project projection endpoints. Requires 6.1.2 and
  6.2.2. See corbusier-api-design.md §Endpoint inventory -- Tasks and §Endpoint
  inventory -- Projects.
  - [ ] Implement `TaskCardDto` and `TaskDetailDto` projections. See
    corbusier-api-design.md §Task domain -- Projection DTOs required by mockup
    pages.
  - [ ] Implement `ProjectCardDto`, `ProjectLandingDto`, and
    `ProjectKanbanDto` projections. See corbusier-api-design.md §Project
    domain -- Projections.
  - [ ] Add dependency graph storage (edges table) and task hierarchy nodes
    (goal, idea, and step). See corbusier-api-design.md §Task domain.
  - [ ] Success criteria: list endpoints return paginated projection DTOs;
    golden DTO fixtures match mockup card schemas.

### 6.3. Conversations, directives, and SSE replay

- [ ] 6.3.1 Add conversation aggregate and message paging. Requires 6.1.2 and
  2.1.2. See corbusier-api-design.md §Conversation domain.
  - [ ] Implement `ConversationAggregate` linking to project and task. See
    corbusier-api-design.md §Conversation domain -- Write-side model.
  - [ ] Backfill conversation rows from existing grouped messages. See
    corbusier-api-design.md §Conversation domain -- Migration.
  - [ ] Expose `ConversationListItemDto` and `ConversationDetailDto` via
    paginated endpoints. See corbusier-api-design.md §Endpoint inventory --
    Conversations and messages.
  - [ ] Success criteria: conversations are listed and retrieved with message
    paging; content parts render through existing serialization.
- [ ] 6.3.2 Persist directives and expose registry endpoints. Requires 3.4.1
  and 6.2.1. See corbusier-api-design.md §Directives domain.
  - [ ] Implement `DirectiveAggregate` scoped to project and tenant. See
    corbusier-api-design.md §Directives domain -- Write-side model.
  - [ ] Seed core directives (`/task`, `/review`) from the existing
    slash-command definitions. See corbusier-api-design.md §Directives
    domain -- Migration.
  - [ ] Success criteria: directives are queryable per project; schema
    validation passes at write time.
- [ ] 6.3.3 Implement SSE replay with `Last-Event-ID` semantics. Requires
  6.1.3. See corbusier-api-design.md §SSE event stream and replay semantics.
  - [ ] Implement conversation-scoped SSE at
    `GET /api/v1/events/conversations/{conversation_id}`. See
    corbusier-api-design.md §Recommended SSE endpoints.
  - [ ] Replay events from the `domain_events` store on reconnect using
    `Last-Event-ID`; emit `stream_reset` when events are no longer retained.
    See
    [HTML Standard: Server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html)
    and
    [MDN: Using server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events)
    for `Last-Event-ID` replay semantics.
  - [ ] Success criteria: reconnecting clients receive replayed events;
    deterministic replay is verified in streaming tests.

### 6.4. Identity, suggestions, and governance

- [ ] 6.4.1 Introduce user aggregate and personnel endpoints. Requires 2.5.1.
  See corbusier-api-design.md §Identity domain.
  - [ ] Implement `UserAggregate` with display name, email, avatar, and role.
    See corbusier-api-design.md §Identity domain -- Write-side model.
  - [ ] Implement `ApiKey` with hashed storage and revocation semantics. See
    corbusier-api-design.md §Identity domain -- Write-side model.
  - [ ] Bootstrap tenant owner as the first user per tenant. See
    corbusier-api-design.md §Identity domain -- Migration.
  - [ ] Success criteria: personnel directory lists tenant users; API keys can
    be created and revoked.
- [ ] 6.4.2 Deliver suggestion lifecycle and accept/dismiss endpoints. Requires
  6.2.1. See corbusier-api-design.md §Suggestions domain.
  - [ ] Implement `SuggestionAggregate` with priority, confidence, tags, and
    rationale. See corbusier-api-design.md §Suggestions domain -- Write-side
    model.
  - [ ] Implement accept (creating a draft task) and dismiss actions. See
    corbusier-api-design.md §Endpoint inventory -- Suggestions.
  - [ ] Success criteria: accepted suggestions produce tasks in backlog;
    `SuggestionCardDto` matches mockup fields.
- [ ] 6.4.3 Add governance CRUD and system endpoint hardening. Requires 4.3.1,
  2.3.2, 2.5.3, and 6.4.1. See corbusier-api-design.md §Governance domain.
  - [ ] Implement `PolicyAggregate` and `HookAggregate` with enable/disable
    lifecycle. See corbusier-api-design.md §Governance domain -- Write-side
    model.
  - [ ] Expose system endpoints for hooks, policies, agents, tool servers, and
    tenant readout. See corbusier-api-design.md §Endpoint inventory -- System.
  - [ ] Add OpenAPI-driven contract tests and multi-tenant isolation tests for
    all phase 5 endpoints. See corbusier-api-design.md §Test strategy.
  - [ ] Success criteria: contract tests validate error schema stability and
    pagination envelope shape; tenant isolation tests pass.

## 7. Deployment and preview environments

### 7.1. Nile Valley-aligned deployment path

- [x] 7.1.1 Add a runtime health endpoint and Kubernetes-ready container.
  - [x] Introduce the health port and Actix Web adapter. See
    `src/health/mod.rs` and `src/health/actix_adapter.rs`.
  - [x] Replace the stub entry point with an HTTP server exposing
    `/health/live` and `/health/ready`. See `src/main.rs`.
  - [x] Add a multi-stage `Dockerfile` and `.dockerignore` for local and CI
    image builds.
  - [x] Success criteria: the release image runs as non-root and exposes
    stable health endpoints on port 8080.
- [x] 7.1.2 Add a Helm chart compatible with local preview and GitOps
  (Git-based operations).
  - [x] Create `charts/corbusier` with deployment, service, ingress,
    ConfigMap, ServiceAccount, PDB, and `ExternalSecret` templates.
  - [x] Add a values schema and local preview values file.
  - [x] Success criteria: the chart can render a hostless local ingress and a
    GitOps-friendly explicit-host ingress from the same values contract.
- [x] 7.1.3 Add a local k3d (Kubernetes in Docker) lifecycle workflow.
  - [x] Create `scripts/local_k8s.py` and the supporting `scripts/local_k8s/`
    package using Cyclopts and `plumbum`.
  - [x] Add `make local-k8s-up`, `local-k8s-status`, `local-k8s-logs`, and
    `local-k8s-down`.
  - [x] Document the design and Nile Valley alignment in
    `docs/local-k8s-preview-design.md`.
  - [x] Success criteria: local preview orchestration is versioned in-repo and
    targets the same chart/image contract intended for Nile Valley overlays.
