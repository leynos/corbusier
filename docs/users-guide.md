# User guide

## Conversation history

Corbusier stores conversation messages as an append-only sequence. History is
retrieved in sequence order via the message repository.

```rust,no_run
use corbusier::context::{CorrelationId, RequestContext, SessionId, UserId};
use corbusier::tenant::TenantId;
use corbusier::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{
        ContentPart, ConversationId, Message, MessageMetadata, Role, SequenceNumber, TextPart,
    },
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;

async fn load_history() -> Result<(), Box<dyn std::error::Error>> {
    let repo = InMemoryMessageRepository::new();
    let clock = DefaultClock;
    let conversation_id = ConversationId::new();
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    let message = Message::builder(conversation_id, Role::User, SequenceNumber::new(1))
        .with_content(ContentPart::Text(TextPart::new("Hello")))
        .with_metadata(MessageMetadata::with_agent_backend("claude"))
        .build(&clock)?;

    repo.store(&ctx, &message).await?;

    let history = repo.find_by_conversation(&ctx, conversation_id).await?;
    assert_eq!(history.len(), 1);
    Ok(())
}
```

## Audit metadata

Message metadata may include audit records for tool calls and agent responses.
Tool call audit entries include a `call_id`, `tool_name`, and `status`, while
agent response audit metadata records a `status` plus optional identifiers such
as `response_id` and `model`.

```rust,no_run
use corbusier::message::domain::{
    AgentResponseAudit, AgentResponseStatus, MessageMetadata, ToolCallAudit, ToolCallStatus,
};

let metadata = MessageMetadata::empty()
    .with_tool_call_audit(ToolCallAudit::new(
        "call-123",
        "read_file",
        ToolCallStatus::Succeeded,
    ))
    .with_agent_response_audit(
        AgentResponseAudit::new(AgentResponseStatus::Completed).with_response_id("resp-456"),
    );

assert_eq!(metadata.tool_call_audits.len(), 1);
assert!(metadata.agent_response_audit.is_some());
```

## Slash command execution

Corbusier provides a slash-command orchestration service that parses commands,
validates typed parameters, renders templates using `minijinja`, and produces
deterministic tool-call plans with auditable metadata.

Supported command grammar:

- `/<command> key=value key2="quoted value"`
- Required parameters are enforced per command schema.
- Unknown parameters are rejected with typed errors.
- Boolean parameter values are case-insensitive (`true`/`false`).
- `number` parameters currently accept integer values only.

The default in-memory registry includes `/task` and `/review` command
definitions.

```rust,no_run
use std::sync::Arc;

use corbusier::message::{
    adapters::memory::InMemorySlashCommandRegistry,
    services::SlashCommandService,
};

fn execute_command() -> Result<(), Box<dyn std::error::Error>> {
    let service = SlashCommandService::new(Arc::new(InMemorySlashCommandRegistry::new()));

    let result = service.execute("/task action=start issue=123")?;

    assert_eq!(result.expansion().command, "/task");
    assert!(!result.planned_tool_calls().is_empty());
    assert_eq!(result.planned_tool_calls().len(), result.tool_call_audits().len());
    Ok(())
}
```

Repeated execution with the same command string and parameter values produces
the same ordered tool-call sequence and call identifiers.

## Issue-to-task creation

Corbusier can create an internal task directly from external issue metadata and
retrieve it by the external issue reference. Issue-origin tasks start in the
`draft` state and record lifecycle timestamps (`created_at`, `updated_at`) at
creation time.

Issue references are unique per tenant, not globally. Two tenants may each map
the same external issue reference to their own internal task without colliding,
while a duplicate within the same tenant is still rejected.

```rust,no_run
use std::sync::Arc;

use corbusier::context::{CorrelationId, RequestContext, SessionId, UserId};
use corbusier::tenant::TenantId;
use corbusier::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::IssueRef,
    services::{CreateTaskFromIssueRequest, TaskLifecycleService},
};
use mockable::DefaultClock;

async fn create_task_from_issue() -> Result<(), Box<dyn std::error::Error>> {
    let service = TaskLifecycleService::new(
        Arc::new(InMemoryTaskRepository::new()),
        Arc::new(DefaultClock),
    );
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    let request = CreateTaskFromIssueRequest::new(
        "github",
        "corbusier/core",
        120,
        "Track issue metadata",
    )
    .with_labels(vec!["feature".to_owned(), "roadmap-1.2.1".to_owned()]);

    let created = service.create_from_issue(&ctx, request).await?;
    let issue_ref = IssueRef::from_parts("github", "corbusier/core", 120)?;
    let fetched = service.find_by_issue_ref(&ctx, &issue_ref).await?;

    assert_eq!(fetched, Some(created));
    Ok(())
}
```

## Tenant-scoped uniqueness

Tenant-owned records are partitioned by `RequestContext::tenant_id`. That
affects both task issue references and backend registration names:

- the same issue reference may exist once per tenant,
- the same backend name may exist once per tenant, and
- lookups return only the caller's tenant-owned records.

When tests or application code reuse an external identifier or backend name for
records that are intended to belong to different tenants, a distinct
`RequestContext` should be used for each tenant.

## Branch and pull request association

Once a task has been created from an issue, a branch reference can be
associated with it. Multiple tasks may share the same branch. Each individual
task has at most one active branch and at most one open pull request.

Associating a pull request with a task automatically transitions the task state
to `in_review`.

```rust,no_run
use std::sync::Arc;

use corbusier::context::{CorrelationId, RequestContext, SessionId, UserId};
use corbusier::tenant::TenantId;
use corbusier::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::{BranchRef, PullRequestRef, TaskState},
    services::{
        AssociateBranchRequest, AssociatePullRequestRequest, CreateTaskFromIssueRequest,
        TaskLifecycleService,
    },
};
use mockable::DefaultClock;

async fn associate_branch_and_pr() -> Result<(), Box<dyn std::error::Error>> {
    let service = TaskLifecycleService::new(
        Arc::new(InMemoryTaskRepository::new()),
        Arc::new(DefaultClock),
    );
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    // Create a task from an issue.
    let task = service
        .create_from_issue(&ctx, CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            200,
            "Implement branch tracking",
        ))
        .await?;

    // Associate a branch with the task.
    let updated = service
        .associate_branch(&ctx, AssociateBranchRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            "feature/branch-tracking",
        ))
        .await?;
    assert!(updated.branch_ref().is_some());

    // Retrieve the task by branch reference.
    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "feature/branch-tracking")?;
    let found = service.find_by_branch_ref(&ctx, &branch_ref).await?;
    assert_eq!(found.len(), 1);

    // Associate a pull request — this transitions the task to in_review.
    let reviewed = service
        .associate_pull_request(&ctx, AssociatePullRequestRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            42,
        ))
        .await?;
    assert_eq!(reviewed.state(), TaskState::InReview);

    // Retrieve the task by pull request reference.
    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 42)?;
    let found = service.find_by_pull_request_ref(&ctx, &pr_ref).await?;
    assert_eq!(found.len(), 1);

    Ok(())
}
```

## Task state transitions

Task state transitions are validated against the domain state machine. Invalid
transitions are rejected with a typed `TaskDomainError::InvalidStateTransition`
error that includes the task ID and the requested `from` and `to` states.

Allowed transitions:

Table 1. Allowed task state transitions.

| From state    | Allowed target states                      |
| ------------- | ------------------------------------------ |
| `draft`       | `in_progress`, `in_review`, `abandoned`    |
| `in_progress` | `in_review`, `paused`, `done`, `abandoned` |
| `in_review`   | `in_progress`, `done`, `abandoned`         |
| `paused`      | `in_progress`, `abandoned`                 |
| `done`        | *(terminal)*                               |
| `abandoned`   | *(terminal)*                               |

```rust,no_run
use std::sync::Arc;

use corbusier::context::{CorrelationId, RequestContext, SessionId, UserId};
use corbusier::tenant::TenantId;
use corbusier::task::{
    adapters::memory::InMemoryTaskRepository,
    domain::{TaskDomainError, TaskState},
    services::{
        CreateTaskFromIssueRequest, TaskLifecycleError, TaskLifecycleService,
        TransitionTaskRequest,
    },
};
use mockable::DefaultClock;

async fn transition_task_states() -> Result<(), Box<dyn std::error::Error>> {
    let service = TaskLifecycleService::new(
        Arc::new(InMemoryTaskRepository::new()),
        Arc::new(DefaultClock),
    );
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    let task = service
        .create_from_issue(&ctx, CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            330,
            "Validate task transitions",
        ))
        .await?;

    let transitioned = service
        .transition_task(&ctx, TransitionTaskRequest::new(task.id(), "in_progress"))
        .await?;
    assert_eq!(transitioned.state(), TaskState::InProgress);

    let invalid = service
        .transition_task(&ctx, TransitionTaskRequest::new(task.id(), "draft"))
        .await;

    assert!(matches!(
        invalid,
        Err(TaskLifecycleError::Domain(
            TaskDomainError::InvalidStateTransition {
                from: TaskState::InProgress,
                to: TaskState::Draft,
                ..
            }
        ))
    ));

    Ok(())
}
```

## Agent backend registration

The `agent_backend` module provides a registry where agent backends declare
their identity and capabilities. Backends are registered by name and can be
listed, looked up, deactivated, and reactivated.

```rust,no_run
use std::sync::Arc;
use corbusier::agent_backend::{
    adapters::memory::InMemoryBackendRegistry,
    services::{BackendRegistryService, RegisterBackendRequest},
};
use corbusier::context::{CorrelationId, RequestContext, SessionId, UserId};
use corbusier::tenant::TenantId;
use mockable::DefaultClock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = BackendRegistryService::new(
        Arc::new(InMemoryBackendRegistry::new()),
        Arc::new(DefaultClock),
    );
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    // Register two backends.
    let claude = service
        .register(
            &ctx,
            RegisterBackendRequest::new(
                "claude_code_sdk",
                "Claude Code SDK",
                "1.0.0",
                "Anthropic",
            )
            .with_capabilities(true, true),
        )
        .await?;

    service
        .register(
            &ctx,
            RegisterBackendRequest::new(
                "codex_cli",
                "Codex CLI",
                "0.9.0",
                "OpenAI",
            )
            .with_capabilities(false, true),
        )
        .await?;

    // List all registered backends.
    let all = service.list_all(&ctx).await?;
    assert_eq!(all.len(), 2);

    // Look up a backend by name.
    let found = service.find_by_name(&ctx, "claude_code_sdk").await?;
    assert!(found.is_some());

    // Deactivate a backend — it no longer appears in active listings.
    service.deactivate(&ctx, claude.id()).await?;
    let active = service.list_active(&ctx).await?;
    assert_eq!(active.len(), 1);

    Ok(())
}
```

## Tenant context

All repository and service operations on tenant-owned data require a
`RequestContext`. This cross-cutting struct carries the tenant identity,
distributed tracing identifiers, and the authenticated principal.

```rust,no_run
use corbusier::context::{
    CausationId, CorrelationId, RequestContext, SessionId, UserId,
};
use corbusier::tenant::TenantId;

fn build_request_context() {
    // Required fields: tenant, correlation, user, session.
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    // Optional causation ID links the operation to its triggering event.
    let ctx_with_cause = ctx.with_causation_id(CausationId::new());
    assert!(ctx_with_cause.causation_id().is_some());
}
```

Tenant identity is modelled separately from user identity. The `TenantSlug`
type enforces Domain Name System (DNS)-label-safe formatting: lowercase
alphanumeric plus hyphens, 1–63 characters, no leading or trailing hyphens, no
consecutive hyphens.

```rust,no_run
use corbusier::tenant::{Tenant, TenantSlug, TenantStatus};
use corbusier::context::UserId;
use mockable::DefaultClock;

fn create_tenant() -> Result<(), Box<dyn std::error::Error>> {
    let clock = DefaultClock;
    let slug = TenantSlug::new("acme-corp")?;
    let owner = UserId::new();
    let tenant = Tenant::new(slug, "Acme Corporation", owner, &clock)?;

    assert_eq!(tenant.slug().as_str(), "acme-corp");
    assert_eq!(tenant.display_name(), "Acme Corporation");
    assert_eq!(tenant.status(), TenantStatus::Active);
    Ok(())
}
```

## HTTP API surface

Corbusier now exposes an initial authenticated HTTP API under `/api/v1`. The
routes are:

- `POST /api/v1/conversations`
- `GET /api/v1/conversations/{conversation_id}/history`
- `POST /api/v1/conversations/{conversation_id}/messages`
- `POST /api/v1/tasks`
- `GET /api/v1/tasks/{task_id}`
- `PUT /api/v1/tasks/{task_id}/state`
- `PUT /api/v1/tasks/{task_id}/branch`
- `PUT /api/v1/tasks/{task_id}/pull-request`
- `GET /api/v1/tools`
- `POST /api/v1/tools/calls`

Every request must send `Authorization: Bearer <JSON Web Token (JWT)>`. The
accepted JWT claims are `sub`, `tenant_id`, `session_id`, `exp`, and optional
`role` plus `tenant_kind`. The current release only accepts
`tenant_kind = user`.

Every response, including errors, uses the same envelope shape:

```json
{
  "success": true,
  "data": {},
  "error": null,
  "metadata": {
    "version": "v1",
    "request_id": "<correlation-id>",
    "timestamp": "<RFC3339 timestamp>"
  }
}
```

Example conversation flow:

```text
POST /api/v1/conversations
Authorization: Bearer <jwt>
```

```text
POST /api/v1/conversations/<conversation_id>/messages
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "role": "user",
  "content": [
    { "type": "text", "text": "Hello over HTTP" }
  ]
}
```

Example task creation:

```text
POST /api/v1/tasks
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "provider": "github",
  "repository": "acme/widgets",
  "issue_number": 42,
  "title": "Fix login flow",
  "description": "Triage the failing callback",
  "labels": ["bug", "p1"],
  "assignees": ["alice"],
  "milestone": "sprint-12"
}
```

Example tool call:

```text
POST /api/v1/tools/calls
Authorization: Bearer <jwt>
Content-Type: application/json

{
  "tool_name": "read_file",
  "parameters": {
    "path": "/tmp/example.txt"
  }
}
```

The current HTTP surface is versioned and authenticated, but it does not yet
provide hardened PostgreSQL tenant isolation for conversation and message
storage. Do not treat it as a complete multi-tenant security boundary until
roadmap items `2.5.2` and `2.5.3` land.

## Model Context Protocol (MCP) server lifecycle management

The `tool_registry` module can register Model Context Protocol (MCP) servers,
start and stop them, refresh health status, and list tools exposed by running
servers. Tool queries are only allowed when a server is in the `running`
lifecycle state.

```rust,no_run
use std::sync::Arc;

use corbusier::context::{CorrelationId, RequestContext, SessionId, UserId};
use corbusier::tenant::TenantId;
use corbusier::tool_registry::{
    adapters::{InMemoryMcpServerHost, memory::InMemoryMcpServerRegistry},
    domain::{McpServerName, McpToolDefinition, McpTransport},
    services::{McpServerLifecycleService, RegisterMcpServerRequest},
};
use mockable::DefaultClock;
use serde_json::json;

async fn manage_mcp_servers() -> Result<(), Box<dyn std::error::Error>> {
    let host = Arc::new(InMemoryMcpServerHost::new());
    host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![McpToolDefinition::new(
            "search_code",
            "Searches the workspace source tree",
            json!({"type": "object", "properties": {"query": {"type": "string"}}}),
        )?],
    )?;

    let service = McpServerLifecycleService::new(
        Arc::new(InMemoryMcpServerRegistry::new()),
        host,
        Arc::new(DefaultClock),
    );

    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    let request = RegisterMcpServerRequest::new(
        "workspace_tools",
        McpTransport::stdio("mcp-server")?,
    );
    let registered = service.register(&ctx, request).await?;
    let started = service.start(&ctx, registered.id()).await?;
    assert_eq!(started.server.lifecycle_state().as_str(), "running");

    let servers = service.list_all(&ctx).await?;
    assert_eq!(servers.len(), 1);

    let tools = service.list_tools(&ctx, started.server.id()).await?;
    assert_eq!(tools.len(), 1);

    let stopped = service.stop(&ctx, started.server.id()).await?;
    assert_eq!(stopped.lifecycle_state().as_str(), "stopped");

    let after_stop = service.list_tools(&ctx, stopped.id()).await;
    assert!(after_stop.is_err());

    Ok(())
}
```

## Tool discovery and call routing

After starting an MCP server via the lifecycle service, use
`ToolDiscoveryRoutingService` to discover its tools, persist them in a durable
catalogue, and route tool calls by name. The service validates parameters
against the tool's input schema, runs pluggable execution governance, routes
the call to the correct hosting server, and records a complete audit trail.

### Discovering tools

Call `discover_and_persist_tools()` after starting a server. This queries the
running server for its tool definitions and persists them in the catalogue.
Rediscovery is idempotent — calling it again replaces the existing entries.

### Querying the catalogue

Call `list_catalog(&ctx)` to see all tools across all registered servers for
the current tenant, including their schemas and availability status. The tool
registry is tenant-scoped: each tenant has an isolated view of tools, and
`tenant_id` is enforced at both the application layer (via `RequestContext`)
and, in the PostgreSQL adapter, via `SET LOCAL app.tenant_id`. The method name
uses the code identifier `catalog`; the surrounding prose uses the
en-GB-oxendict spelling "catalogue".

### Calling a tool

Call `call_tool()` with a `ToolCallRequest` containing the tool name and
parameters. The service:

1. Resolves the tool name to a catalogue entry.
2. Checks that the tool is available (its hosting server is running).
3. Validates parameters against the tool's declared input schema.
4. Runs the configured governance adapter before execution. The default
   `StubGovernance::allowing()` adapter permits all calls, while hook-backed
   governance can deny a call before the Model Context Protocol (MCP) host runs
   it.
5. Routes the call to the correct MCP server when governance permits it.
6. Records the existing tool-call audit trail entry with outcome, duration, and
   any stderr.
7. Runs post-tool-use governance observation after the call completes.

When hook-backed governance is configured, `ToolCallRequest` can carry a
`TaskId` and `ConversationId` through its execution scope. Policy audit
outcomes are then queryable from the hook engine by task, conversation, and
hook event (`TriggerContextId`) without inspecting arbitrary JSON payloads.

### Tool availability lifecycle

When a server is stopped, call `mark_tools_unavailable()` to flag all its tools
as unavailable. Subsequent `call_tool()` requests for those tools are rejected
with `ToolUnavailable`. When the server is restarted and tools are
rediscovered, they become available again.

### Stderr log capture

Startup stderr (from `McpServerHost::start`) and per-tool-call stderr are
automatically captured and stored via the `ToolLogStore` port. The default
adapter uses the Rust `object_store` crate with a configurable backend (local
filesystem or in-memory for tests). Log references are recorded in the audit
trail's `stderr_log_path` field.

The `LogRetentionPolicy` controls log rotation:

- `max_bytes_per_log`: 10 MiB default; logs exceeding this are truncated.
- `max_logs_per_server`: 100 default; oldest logs are deleted first.
- `retention_period`: 7 days default; expired logs are swept on startup
  and on demand via `sweep_expired_logs()`.

```rust,no_run
use std::sync::Arc;

use corbusier::context::{CorrelationId, RequestContext, SessionId, UserId};
use corbusier::tenant::TenantId;
use corbusier::tool_registry::{
    adapters::{
        InMemoryMcpServerHost, ObjectStoreLogAdapter, StubGovernance,
        memory::{InMemoryMcpServerRegistry, InMemoryToolCatalog},
    },
    domain::{
        LogRetentionPolicy, McpServerName, McpToolDefinition, McpTransport,
        ToolCallRequest,
    },
    services::{
        McpServerLifecycleService, RegisterMcpServerRequest, ServicePorts,
        ToolDiscoveryRoutingService,
    },
};
use mockable::DefaultClock;
use serde_json::json;

async fn discover_and_call_tools() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(InMemoryMcpServerRegistry::new());
    let host = Arc::new(InMemoryMcpServerHost::new());
    let catalog = Arc::new(InMemoryToolCatalog::new());
    let clock = Arc::new(DefaultClock);

    // Configure the test host with a tool and its call result.
    host.set_tool_catalog(
        McpServerName::new("workspace_tools")?,
        vec![McpToolDefinition::new(
            "read_file",
            "Reads a file from the workspace",
            json!({"type": "object", "required": ["path"],
                   "properties": {"path": {"type": "string"}}}),
        )?],
    )?;
    host.set_tool_call_result(
        McpServerName::new("workspace_tools")?,
        "read_file",
        json!({"content": "hello world"}),
    )?;

    // Create the lifecycle and discovery services.
    let lifecycle = McpServerLifecycleService::new(
        registry.clone(), host.clone(), clock.clone(),
    );
    let discovery = ToolDiscoveryRoutingService::new(
        ServicePorts {
            catalog,
            registry,
            host,
            governance: Arc::new(StubGovernance::allowing()),
            log_store: Arc::new(ObjectStoreLogAdapter::in_memory()),
        },
        LogRetentionPolicy::default(),
        clock,
    );

    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    // Register, start, and discover tools.
    let request = RegisterMcpServerRequest::new(
        "workspace_tools", McpTransport::stdio("mcp-server")?,
    );
    let registered = lifecycle.register(&ctx, request).await?;
    lifecycle.start(&ctx, registered.id()).await?;
    let entries = discovery
        .discover_and_persist_tools(&ctx, registered.id())
        .await?;
    assert_eq!(entries.len(), 1);

    // Call a tool by name -- routing resolves the hosting server.
    let call = ToolCallRequest::new(
        "read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock,
    );
    let result = discovery.call_tool(&ctx, &call).await?;
    assert!(result.outcome().is_success());

    // Stop the server and mark tools unavailable.
    lifecycle.stop(&ctx, registered.id()).await?;
    discovery.mark_tools_unavailable(&ctx, registered.id()).await?;

    // Subsequent calls are rejected.
    let retry = ToolCallRequest::new(
        "read_file", json!({"path": "/tmp/test.txt"}), &DefaultClock,
    );
    assert!(discovery.call_tool(&ctx, &retry).await.is_err());

    Ok(())
}
```

## Agent turn orchestration and sessions

Corbusier orchestrates agent turns through a single service path that validates
the target backend, resolves or rotates a backend runtime session, executes the
turn, and routes tool calls in deterministic order. Session continuity is
tracked per `(tenant_id, backend_id, conversation_id)` until the session
expires, with every repository query binding `RequestContext.tenant_id` so
tenant isolation is enforced for reuse and rotation.

When a stored session is still active within the current tenant, it is reused.
When expired, the session is marked expired and a new runtime session is
created automatically. The session repository commits a `reserved` slot row for
the current tenant before the runtime session is created, so expiry detection,
replacement-session claiming, and subsequent reuse all remain tenant-scoped and
happen at the database boundary before any external runtime call is made.

```rust,no_run
use std::sync::Arc;

use corbusier::agent_backend::{
    adapters::memory::{
        InMemoryAgentRuntime, InMemoryBackendRegistry, InMemoryToolRouter,
        InMemoryTurnSessionRepository,
    },
    domain::{BackendId, ToolCallRequest, TurnExecutionRequest, TurnExecutionResult},
    ports::BackendRegistryRepository,
    services::{
        AgentTurnOrchestratorPorts, AgentTurnOrchestratorService, ExecuteAgentTurnRequest,
    },
};
use chrono::Duration;
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use mockable::DefaultClock;
use serde_json::json;
use uuid::Uuid;

async fn execute_orchestrated_turn() -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(InMemoryBackendRegistry::new());
    let sessions = Arc::new(InMemoryTurnSessionRepository::new());
    let runtime = Arc::new(InMemoryAgentRuntime::new());
    let router = Arc::new(InMemoryToolRouter::new());
    let clock = Arc::new(DefaultClock);
    let ctx = RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );

    // Register one backend used for orchestration.
    let backend = corbusier::agent_backend::services::BackendRegistryService::new(
        registry.clone(),
        clock.clone(),
    )
    .register(
        &ctx,
        corbusier::agent_backend::services::RegisterBackendRequest::new(
            "claude_code_sdk",
            "Claude Code SDK",
            "1.0.0",
            "Anthropic",
        )
        .with_capabilities(true, true),
    )
    .await?;

    // Configure runtime and tool router behaviour for demonstration.
    runtime.queue_turn_result(TurnExecutionResult::new(
        "assistant response",
        vec![ToolCallRequest::new("search_docs", json!({"query": "roadmap"}))?],
    ))?;
    router.set_tool_response("search_docs", json!({"matches": 3}))?;

    let orchestrator = AgentTurnOrchestratorService::with_config(
        AgentTurnOrchestratorPorts {
            backend_registry: registry,
            turn_sessions: sessions,
            runtime,
            tool_router: router,
            clock,
        },
        corbusier::agent_backend::services::AgentTurnOrchestratorConfig::new(
            Duration::minutes(5),
        )?,
    );

    let response = orchestrator
        .execute_turn(
            &ctx,
            ExecuteAgentTurnRequest::new(
                BackendId::from_uuid(backend.id().into_inner()),
                TurnExecutionRequest::new(Uuid::new_v4(), "Please summarize this", Vec::new()),
            ),
        )
        .await?;

    assert_eq!(response.tool_results().len(), 1);
    assert!(!response.rotated_session());
    Ok(())
}
```
