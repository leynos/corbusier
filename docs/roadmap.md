# Development roadmap

The roadmap translates the Corbusier design into phased, measurable delivery
steps. Work is ordered by dependency and avoids time-based commitments, while
staying within the in-scope capabilities defined in corbusier-design.md.

## 1. Core orchestration foundation

### 1.1. Conversation management

- [x] 1.1.1 Implement the canonical message format and validation. See
  corbusier-design.md §2.2.1.
  - [x] Define user, assistant, and tool event schemas. See
    corbusier-design.md §2.2.1.
  - [x] Add versioned schema validation at ingestion boundaries. See
    corbusier-design.md §4.4.2.1.
  - [x] Success criteria: all stored messages conform to the canonical schema.
- [x] 1.1.2 Persist message history with audit trails. See
  corbusier-design.md §2.2.1 and §6.2.3.
  - [x] Implement conversation history persistence with immutable ordering. See
    corbusier-design.md §6.2.3.
  - [x] Store audit metadata for tool calls and agent responses. See
    corbusier-design.md §2.1.1.
  - [x] Success criteria: conversation history is queryable by conversation id
    with complete audit metadata.
- [x] 1.1.3 Preserve context across agent handoffs. See
  corbusier-design.md §2.2.1 and §4.1.1.1.
  - [x] Persist handoff metadata between agent turns. See
    corbusier-design.md §4.2.1.1.
  - [x] Maintain context window snapshots per agent session. See
    corbusier-design.md §2.2.1.
  - [x] Success criteria: every handoff references the prior turn and tool
    calls used to reach the handoff.

### 1.2. Task lifecycle management

- [x] 1.2.1 Implement issue-to-task creation and tracking. See
  corbusier-design.md §2.2.2.
  - [x] Map external issue metadata into internal task records. See
    corbusier-design.md §2.2.2.
  - [x] Generate task identifiers and lifecycle timestamps. See
    corbusier-design.md §4.3.1.2.
  - [x] Success criteria: tasks can be created from issues and retrieved by
    external issue reference.
- [x] 1.2.2 Associate branches and pull requests with tasks. Requires 1.2.1. See
  corbusier-design.md §2.2.2.
  - [x] Persist branch identifiers alongside task records. See
    corbusier-design.md §2.2.2.
  - [x] Map pull request identifiers to task state updates. See
    corbusier-design.md §2.2.2 and §4.1.1.2.
  - [x] Success criteria: task records include branch and pull request
    references for all linked work items.
- [x] 1.2.3 Enforce task state transitions with validation. Requires
      1.2.1.[^1]
  - [x] Define allowed transitions and terminal states.[^2]
  - [x] Reject invalid transitions with typed errors.[^3]
  - [x] Success criteria: unit tests cover all 36 source/target transition
    pairs, invalid transitions return typed error variants, and terminal states
    reject all outgoing transitions in tested paths.

[^1]: [docs/corbusier-design.md](docs/corbusier-design.md) §4.3.1.2 and
  §4.4.1.1.
[^2]: [docs/corbusier-design.md](docs/corbusier-design.md) §4.3.1.2.
[^3]: [docs/corbusier-design.md](docs/corbusier-design.md) §4.4.1.1.

### 1.3. Agent backend orchestration

- [x] 1.3.1 Implement agent backend registration and discovery. See
  corbusier-design.md §2.2.3.
  - [x] Define backend capability metadata and registration flow. See
    corbusier-design.md §2.2.3.
  - [x] Persist backend registry entries in the persistence layer. See
    corbusier-design.md §6.2.3.
  - [x] Success criteria: at least two backends can be registered and listed
    via the registry API.
- [x] 1.3.2 Orchestrate agent turn execution and sessions. Requires 1.3.1 and
  1.1.1. See corbusier-design.md §2.2.3.
  - [x] Coordinate turn execution with tool calls and responses. See
    corbusier-design.md §4.2.1.1.
  - [x] Maintain session state and expiry rules. See
    corbusier-design.md §2.2.3.
  - [x] Success criteria: agent turns execute with consistent tool routing and
    session continuity.
- [ ] 1.3.3 Translate tool schemas per backend. Requires 2.1.1. See
  corbusier-design.md §2.2.3 and §2.2.4.
  - [ ] Implement schema translation mapping for each backend. See
    corbusier-design.md §2.2.3.
  - [ ] Validate translated schemas against MCP tool definitions. See
    corbusier-design.md §2.2.4.
  - [ ] Success criteria: tool schemas are accepted by each registered backend
    without manual edits.

### 1.4. Slash command system

- [x] 1.4.1 Deliver slash command parsing and template execution. Requires
  1.1.1. See corbusier-design.md §2.1.1.
  - [x] Implement command parser and registry. See corbusier-design.md §2.1.1.
  - [x] Add template expansion and parameter validation. See
    corbusier-design.md §2.1.1.
  - [x] Success criteria: commands produce deterministic tool call sequences
    with auditable records.

### 1.5. Tenant context and identity isolation

- [x] 1.5.1 Establish tenant primitives and request context plumbing. See
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
- [ ] 1.5.2 Deliver tenant-aware schema and constraints. Requires 1.5.1. See
  corbusier-design.md §6.2.1 and §6.2.2.
  - [ ] Create `tenants` and add `tenant_id` to tenant-owned tables. See
    corbusier-design.md §2.2.5.
  - [ ] Scope task issue-origin uniqueness and backend registration uniqueness
    by tenant. See corbusier-design.md §2.2.2 and §2.2.3.
  - [ ] Enforce parent/child tenant consistency with composite foreign keys.
    See corbusier-design.md §6.2.1.
  - [ ] Success criteria: same issue reference and backend name can exist in
    multiple tenants without collisions.
- [ ] 1.5.3 Enforce tenant boundaries in adapters and PostgreSQL. Requires
  1.5.2. See corbusier-design.md §6.2.2 and §6.2.3. Note: 1.5.1 added
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
- [ ] 1.5.4 Prove multi-tenant isolation with two-tenant scenarios. Requires
  1.5.3. See corbusier-design.md §2.2.5.
  - [ ] Add tests where two tenants share the same external issue identifiers
    without violating constraints. See corbusier-design.md §2.2.5.
  - [ ] Add tests where two tenants register the same backend name without
    collisions. See corbusier-design.md §2.2.5.
  - [ ] Add negative tests showing tenant A context cannot retrieve tenant B
    records. See corbusier-design.md §2.2.5.
  - [ ] Success criteria: isolation tests fail when tenant scoping is missing
    and pass when tenant context and RLS are correctly applied.

## 2. Tool plane and workflow governance

### 2.1. MCP hosting and tool registry

- [x] 2.1.1 Implement MCP server lifecycle management. Requires 1.3.1. See
  corbusier-design.md §2.2.4 and §6.1.4.
  - [x] Support MCP server start, stop, and health reporting. See
    corbusier-design.md §2.2.4.
  - [x] Register MCP servers in the tool registry. See
    corbusier-design.md §2.2.4.
  - [x] Success criteria: MCP servers can be started, listed, and queried for
    available tools.
- [x] 2.1.2 Deliver tool discovery and routing. Requires 2.1.1. See
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

### 2.2. Weaver file editing integration

- [ ] 2.2.1 Enforce Weaver as the authoritative file editor. Requires 2.1.1. See
  corbusier-design.md §2.1.2.
  - [ ] Disable direct file edits from agent backends. See
    corbusier-design.md §2.1.2.
  - [ ] Persist ChangeSet metadata from Weaver. See corbusier-design.md §2.1.2.
  - [ ] Success criteria: all file changes are represented as Weaver ChangeSets
    with diff metadata.

### 2.3. Hook engine and policy enforcement

- [x] 2.3.1 Implement hook engine execution. Requires 1.2.3 and 2.1.2. See
  corbusier-design.md §2.1.3 and §6.3.3.
  - [x] Define hook triggers for turn start/end, tool use before/after, and
    pre- and post-commit, pre- and post-merge, pre- and post-pull, pre- and
    post-push, and pre- and post-deploy events. See
    corbusier-design.md §2.1.3.
  - [x] Execute hooks with structured outcomes and logs. See
    corbusier-design.md §6.3.3.
  - [x] Success criteria: hook execution results are recorded for every
    configured trigger.
- [ ] 2.3.2 Add policy enforcement and audit capture. Requires 2.3.1. See
  corbusier-design.md §2.1.3 and §6.4.2.5.
  - [ ] Implement policy evaluation at enforcement points. See
    corbusier-design.md §6.4.2.4.
  - [ ] Persist policy violations and audit events. See
    corbusier-design.md §6.4.2.5.
  - [ ] Success criteria: policy enforcement outcomes are queryable by task,
    conversation, and hook event.

### 2.4. Encapsulation and workspace management

- [ ] 2.4.1 Implement workspace encapsulation for tool execution. Requires
  2.1.1. See corbusier-design.md §2.1.3 and §6.2.4.
  - [ ] Provision Podbot-backed workspaces per task. See
    corbusier-design.md §6.2.4.
  - [ ] Enforce workspace isolation for tool and VCS operations. See
    corbusier-design.md §6.4.4.3.
  - [ ] Success criteria: tool execution occurs only within encapsulated
    workspaces with auditable boundaries.

## 3. External integrations and interfaces

### 3.1. VCS integration and review ingestion

- [ ] 3.1.1 Deliver VCS adapter for GitHub and GitLab. Requires 1.2.2. See
  corbusier-design.md §2.1.4 and §6.3.1.
  - [ ] Implement branch, pull request, and issue operations. See
    corbusier-design.md §2.1.4.
  - [ ] Map VCS events into task lifecycle updates. See
    corbusier-design.md §4.1.1.2.
  - [ ] Success criteria: tasks remain synchronised with VCS state transitions.
- [ ] 3.1.2 Implement review ingestion workflows. Requires 3.1.1. See
  corbusier-design.md §2.1.4 and §6.3.2.
  - [ ] Ingest review comments and map them to tasks. See
    corbusier-design.md §6.3.2.
  - [ ] Store review metadata for audit and reporting. See
    corbusier-design.md §2.1.3.
  - [ ] Success criteria: review comments are attached to the relevant task and
    conversation records.

### 3.2. HTTP API surface

- [ ] 3.2.1 Expose core HTTP APIs for conversations and tasks. Requires 1.2.3.
  See corbusier-design.md §2.1.4 and §6.2.1.
  - [ ] Implement conversation, task, and tool endpoints. See
    corbusier-design.md §6.2.1.
  - [ ] Apply API versioning and authentication requirements. See
    corbusier-design.md §6.3.1.5 and §6.4.1.
  - [ ] Success criteria: API endpoints return versioned responses and enforce
    authentication.

### 3.3. Real-time event streaming

- [ ] 3.3.1 Provide real-time event streaming for orchestration updates.
  Requires 3.2.1. See corbusier-design.md §2.1.4 and §6.2.2.
  - [ ] Implement event schema for conversation and task updates. See
    corbusier-design.md §6.2.2.
  - [ ] Publish events through SSE or equivalent transport. See
    corbusier-design.md §6.2.2 and §7.4.2.
  - [ ] Success criteria: subscribers receive ordered event streams with retry
    support.

### 3.4. Operator and developer user interfaces

- [ ] 3.4.1 Deliver the task management interface. Requires 3.2.1. See
  corbusier-design.md §7.2.1 and §7.5.1.
  - [ ] Implement task list, status, and milestone views. See
    corbusier-design.md §7.5.1.
  - [ ] Add task detail panels with audit history. See
    corbusier-design.md §7.2.1.
  - [ ] Success criteria: tasks can be filtered by status, owner, and milestone.
- [ ] 3.4.2 Deliver the conversation management interface. Requires 3.2.1. See
  corbusier-design.md §7.2.2 and §7.5.2.
  - [ ] Render canonical message history with tool call metadata. See
    corbusier-design.md §7.2.2.
  - [ ] Provide agent handoff visibility and annotations. See
    corbusier-design.md §7.5.2.
  - [ ] Success criteria: conversations show complete message history with
    tool call details.

## 4. Reliability, security, and operations

### 4.1. Security and access control

- [ ] 4.1.1 Implement authentication and session management. Requires 3.2.1. See
  corbusier-design.md §6.4.1.
  - [ ] Add identity management and token handling. See
    corbusier-design.md §6.4.1.1 and §6.4.1.4.
  - [ ] Enforce session expiry and rotation policies. See
    corbusier-design.md §6.4.1.3.
  - [ ] Success criteria: authenticated sessions expire and rotate according to
    policy.
- [ ] 4.1.2 Implement RBAC and resource authorisation. Requires 4.1.1. See
  corbusier-design.md §6.4.2.
  - [ ] Define role hierarchy and permissions. See
    corbusier-design.md §6.4.2.1 and §6.4.2.2.
  - [ ] Enforce authorisation checks at policy enforcement points. See
    corbusier-design.md §6.4.2.4.
  - [ ] Success criteria: all API endpoints require explicit authorisation for
    protected resources.

### 4.2. Observability and monitoring

- [ ] 4.2.1 Implement metrics, logging, and tracing pipelines. Requires 2.3.1.
  See corbusier-design.md §6.5.1 and §6.5.4.
  - [ ] Instrument core services with metrics and traces. See
    corbusier-design.md §6.5.1.1 and §6.5.1.3.
  - [ ] Centralise log aggregation with correlation ids. See
    corbusier-design.md §6.5.1.2.
  - [ ] Success criteria: metrics, traces, and logs share a common correlation
    identifier per request.
- [ ] 4.2.2 Deliver monitoring dashboards and alerting. Requires 4.2.1. See
  corbusier-design.md §6.5.1.5 and §8.5.
  - [ ] Create dashboards for task completion, agent utilisation, and tool
    execution. See corbusier-design.md §8.5.1.
  - [ ] Define alert thresholds and routing. See
    corbusier-design.md §6.5.1.4 and §6.5.3.1.
  - [ ] Success criteria: alerting covers latency, error rate, and availability
    thresholds.

### 4.3. Testing and quality gates

- [ ] 4.3.1 Implement automated unit, integration, and end-to-end test suites.
  Requires 1.2.3 and 2.3.1. See corbusier-design.md §6.6.1.
  - [ ] Add unit tests for domain services and ports. See
    corbusier-design.md §6.6.1.1.
  - [ ] Add integration tests for VCS, tool, and agent adapters. See
    corbusier-design.md §6.6.1.2.
  - [ ] Success criteria: test suites cover critical workflows without manual
    setup.
- [ ] 4.3.2 Enforce CI quality gates for formatting, linting, and test runs.
  Requires 4.3.1. See corbusier-design.md §6.6.2 and §8.4.1.
  - [ ] Configure CI to run formatter, linter, and test pipelines. See
    corbusier-design.md §8.4.1.
  - [ ] Block merges on failed quality gates. See
    corbusier-design.md §6.6.2.1.
  - [ ] Success criteria: no mainline merge occurs without passing quality
    gates.

### 4.4. Deployment and resilience

- [ ] 4.4.1 Deliver containerised deployment and rollback workflows. Requires
  4.3.2. See corbusier-design.md §8.2 and §8.4.3.
  - [ ] Build multi-stage container images with security scanning. See
    corbusier-design.md §8.2.2 and §8.2.5.
  - [ ] Implement deployment and rollback procedures. See
    corbusier-design.md §8.4.2 and §8.4.3.
  - [ ] Success criteria: deployments support automated rollback on failed
    validation.

### 4.5. Performance and scalability

- [ ] 4.5.1 Validate performance, scalability, and SLA targets. Requires 4.2.2.
  See corbusier-design.md §4.5 and §6.2.4.
  - [ ] Execute performance tests against response time targets. See
    corbusier-design.md §4.5.1.1.
  - [ ] Validate horizontal scaling behaviour under concurrency targets. See
    corbusier-design.md §4.5.3.1.
  - [ ] Success criteria: response latency remains under target thresholds and
    scaling tests meet concurrency goals.

## 5. Front-end API surface and data-model extensions

### 5.1. API contracts and scaffolding

- [ ] 5.1.1 Publish versioned OpenAPI specification and central error schema.
  Requires 3.2.1. See corbusier-api-design.md §HTTP API surface, pagination,
  SSE, and error contracts.
  - [ ] Define `ErrorCode` enum and `Error` response schema compatible with
    Wildside. See corbusier-api-design.md §Error and validation contract.
  - [ ] Generate `/api/v1` OpenAPI document covering error, pagination, and
    auth contracts.
  - [ ] Success criteria: error responses are validated against the schema in
    contract tests.
- [ ] 5.1.2 Implement reusable keyset pagination crate. Requires 5.1.1. See
  corbusier-api-design.md §Pagination semantics.
  - [ ] Implement cursor encoding and decoding with opaque tokens. See
    corbusier-api-design.md §Pagination semantics.
  - [ ] Implement `Paginated<T>` envelope with `data`, `limit`, and hypermedia
    `links` (self, next, and prev). See corbusier-api-design.md §Pagination
    semantics.
  - [ ] Success criteria: pagination envelope shape matches TanStack Query
    infinite query expectations; absence of `next` indicates end-of-list.
- [ ] 5.1.3 Add domain event persistence and SSE endpoint skeleton. Requires
  5.1.1. See corbusier-api-design.md §SSE event stream and replay semantics.
  - [ ] Create `domain_events` table (`tenant_id`, `aggregate_id`,
    `aggregate_type`, `event_type`, `event_data`, and `occurred_at`). See
    corbusier-api-design.md §Replay storage.
  - [ ] Implement SSE endpoint skeleton at `GET /api/v1/events` with event
    identifier emission and `Last-Event-ID` parsing.[^4]
  - [ ] Success criteria: SSE endpoint emits well-formed events with stable
    identifiers; `Last-Event-ID` header is parsed on reconnect.

### 5.2. Project and task read models

- [ ] 5.2.1 Introduce project aggregate and bootstrap default projects.
  Requires 1.5.1. See corbusier-api-design.md §Project domain.
  - [ ] Implement `ProjectAggregate` with slug, localized name and
    description, lead, date range, status, and team membership. See
    corbusier-api-design.md §Project domain.
  - [ ] Seed a default project per tenant and attach existing tasks. See
    corbusier-api-design.md §Project domain -- Migration.
  - [ ] Success criteria: projects can be created, listed, and retrieved by
    slug with tenant scoping.
- [ ] 5.2.2 Extend task aggregate with front-end fields. Requires 5.2.1 and
  1.2.3. See corbusier-api-design.md §Task domain.
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
- [ ] 5.2.3 Deliver task and project projection endpoints. Requires 5.1.2 and
  5.2.2. See corbusier-api-design.md §Endpoint inventory -- Tasks and §Endpoint
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

### 5.3. Conversations, directives, and SSE replay

- [ ] 5.3.1 Add conversation aggregate and message paging. Requires 5.1.2 and
  1.1.2. See corbusier-api-design.md §Conversation domain.
  - [ ] Implement `ConversationAggregate` linking to project and task. See
    corbusier-api-design.md §Conversation domain -- Write-side model.
  - [ ] Backfill conversation rows from existing grouped messages. See
    corbusier-api-design.md §Conversation domain -- Migration.
  - [ ] Expose `ConversationListItemDto` and `ConversationDetailDto` via
    paginated endpoints. See corbusier-api-design.md §Endpoint inventory --
    Conversations and messages.
  - [ ] Success criteria: conversations are listed and retrieved with message
    paging; content parts render through existing serialization.
- [ ] 5.3.2 Persist directives and expose registry endpoints. Requires 1.4.1
  and 5.2.1. See corbusier-api-design.md §Directives domain.
  - [ ] Implement `DirectiveAggregate` scoped to project and tenant. See
    corbusier-api-design.md §Directives domain -- Write-side model.
  - [ ] Seed core directives (`/task`, `/review`) from the existing
    slash-command definitions. See corbusier-api-design.md §Directives
    domain -- Migration.
  - [ ] Success criteria: directives are queryable per project; schema
    validation passes at write time.
- [ ] 5.3.3 Implement SSE replay with `Last-Event-ID` semantics. Requires
  5.1.3. See corbusier-api-design.md §SSE event stream and replay semantics.
  - [ ] Implement conversation-scoped SSE at
    `GET /api/v1/events/conversations/{conversation_id}`. See
    corbusier-api-design.md §Recommended SSE endpoints.
  - [ ] Replay events from the `domain_events` store on reconnect using
    `Last-Event-ID`; emit `stream_reset` when events are no longer
    retained.[^4]
  - [ ] Success criteria: reconnecting clients receive replayed events;
    deterministic replay is verified in streaming tests.

### 5.4. Identity, suggestions, and governance

- [ ] 5.4.1 Introduce user aggregate and personnel endpoints. Requires 1.5.1.
  See corbusier-api-design.md §Identity domain.
  - [ ] Implement `UserAggregate` with display name, email, avatar, and role.
    See corbusier-api-design.md §Identity domain -- Write-side model.
  - [ ] Implement `ApiKey` with hashed storage and revocation semantics. See
    corbusier-api-design.md §Identity domain -- Write-side model.
  - [ ] Bootstrap tenant owner as the first user per tenant. See
    corbusier-api-design.md §Identity domain -- Migration.
  - [ ] Success criteria: personnel directory lists tenant users; API keys can
    be created and revoked.
- [ ] 5.4.2 Deliver suggestion lifecycle and accept/dismiss endpoints. Requires
  5.2.1. See corbusier-api-design.md §Suggestions domain.
  - [ ] Implement `SuggestionAggregate` with priority, confidence, tags, and
    rationale. See corbusier-api-design.md §Suggestions domain -- Write-side
    model.
  - [ ] Implement accept (creating a draft task) and dismiss actions. See
    corbusier-api-design.md §Endpoint inventory -- Suggestions.
  - [ ] Success criteria: accepted suggestions produce tasks in backlog;
    `SuggestionCardDto` matches mockup fields.
- [ ] 5.4.3 Add governance CRUD and system endpoint hardening. Requires 2.3.1
  and 5.4.1. See corbusier-api-design.md §Governance domain.
  - [ ] Implement `PolicyAggregate` and `HookAggregate` with enable/disable
    lifecycle. See corbusier-api-design.md §Governance domain -- Write-side
    model.
  - [ ] Expose system endpoints for hooks, policies, agents, tool servers, and
    tenant readout. See corbusier-api-design.md §Endpoint inventory -- System.
  - [ ] Add OpenAPI-driven contract tests and multi-tenant isolation tests for
    all phase 5 endpoints. See corbusier-api-design.md §Test strategy.
  - [ ] Success criteria: contract tests validate error schema stability and
    pagination envelope shape; tenant isolation tests pass.

[^4]: See
      [HTML Standard: Server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html)
  and
  [MDN: Using server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events)
   for `Last-Event-ID` replay semantics.

## 6. Deployment and preview environments

### 6.1. Nile Valley-aligned deployment path

- [x] 6.1.1 Add a runtime health endpoint and Kubernetes-ready container.
  - [x] Introduce the health port and Actix Web adapter. See
    `src/health/mod.rs` and `src/health/actix_adapter.rs`.
  - [x] Replace the stub entry point with an HTTP server exposing
    `/health/live` and `/health/ready`. See `src/main.rs`.
  - [x] Add a multi-stage `Dockerfile` and `.dockerignore` for local and CI
    image builds.
  - [x] Success criteria: the release image runs as non-root and exposes
    stable health endpoints on port 8080.
- [x] 6.1.2 Add a Helm chart compatible with local preview and GitOps.
  - [x] Create `charts/corbusier` with deployment, service, ingress,
    ConfigMap, ServiceAccount, PDB, and `ExternalSecret` templates.
  - [x] Add a values schema and local preview values file.
  - [x] Success criteria: the chart can render a hostless local ingress and a
    GitOps-friendly explicit-host ingress from the same values contract.
- [x] 6.1.3 Add a local k3d lifecycle workflow.
  - [x] Create `scripts/local_k8s.py` and the supporting `scripts/local_k8s/`
    package using Cyclopts and `plumbum`.
  - [x] Add `make local-k8s-up`, `local-k8s-status`, `local-k8s-logs`, and
    `local-k8s-down`.
  - [x] Document the design and Nile Valley alignment in
    `docs/local-k8s-preview-design.md`.
  - [x] Success criteria: local preview orchestration is versioned in-repo and
    targets the same chart/image contract intended for Nile Valley overlays.
