# User guide

## Conversation history

Corbusier stores conversation messages as an append-only sequence. History is
retrieved in sequence order via the message repository.

```rust,no_run
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

    let message = Message::builder(conversation_id, Role::User, SequenceNumber::new(1))
        .with_content(ContentPart::Text(TextPart::new("Hello")))
        .with_metadata(MessageMetadata::with_agent_backend("claude"))
        .build(&clock)?;

    repo.store(&message).await?;

    let history = repo.find_by_conversation(conversation_id).await?;
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

```rust,no_run
use std::sync::Arc;

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

    let request = CreateTaskFromIssueRequest::new(
        "github",
        "corbusier/core",
        120,
        "Track issue metadata",
    )
    .with_labels(vec!["feature".to_owned(), "roadmap-1.2.1".to_owned()]);

    let created = service.create_from_issue(request).await?;
    let issue_ref = IssueRef::from_parts("github", "corbusier/core", 120)?;
    let fetched = service.find_by_issue_ref(&issue_ref).await?;

    assert_eq!(fetched, Some(created));
    Ok(())
}
```

## Branch and pull request association

Once a task has been created from an issue, a branch reference can be
associated with it. Multiple tasks may share the same branch. Each individual
task has at most one active branch and at most one open pull request.

Associating a pull request with a task automatically transitions the task state
to `in_review`.

```rust,no_run
use std::sync::Arc;

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

    // Create a task from an issue.
    let task = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            200,
            "Implement branch tracking",
        ))
        .await?;

    // Associate a branch with the task.
    let updated = service
        .associate_branch(AssociateBranchRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            "feature/branch-tracking",
        ))
        .await?;
    assert!(updated.branch_ref().is_some());

    // Retrieve the task by branch reference.
    let branch_ref = BranchRef::from_parts("github", "corbusier/core", "feature/branch-tracking")?;
    let found = service.find_by_branch_ref(&branch_ref).await?;
    assert_eq!(found.len(), 1);

    // Associate a pull request — this transitions the task to in_review.
    let reviewed = service
        .associate_pull_request(AssociatePullRequestRequest::new(
            task.id(),
            "github",
            "corbusier/core",
            42,
        ))
        .await?;
    assert_eq!(reviewed.state(), TaskState::InReview);

    // Retrieve the task by pull request reference.
    let pr_ref = PullRequestRef::from_parts("github", "corbusier/core", 42)?;
    let found = service.find_by_pull_request_ref(&pr_ref).await?;
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

    let task = service
        .create_from_issue(CreateTaskFromIssueRequest::new(
            "github",
            "corbusier/core",
            330,
            "Validate task transitions",
        ))
        .await?;

    let transitioned = service
        .transition_task(TransitionTaskRequest::new(task.id(), "in_progress"))
        .await?;
    assert_eq!(transitioned.state(), TaskState::InProgress);

    let invalid = service
        .transition_task(TransitionTaskRequest::new(task.id(), "draft"))
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
use mockable::DefaultClock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = BackendRegistryService::new(
        Arc::new(InMemoryBackendRegistry::new()),
        Arc::new(DefaultClock),
    );

    // Register two backends.
    let claude = service
        .register(
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
    let all = service.list_all().await?;
    assert_eq!(all.len(), 2);

    // Look up a backend by name.
    let found = service.find_by_name("claude_code_sdk").await?;
    assert!(found.is_some());

    // Deactivate a backend — it no longer appears in active listings.
    service.deactivate(claude.id()).await?;
    let active = service.list_active().await?;
    assert_eq!(active.len(), 1);

    Ok(())
}
```

## MCP server lifecycle management

The `tool_registry` module can register MCP servers, start and stop them,
refresh health status, and list tools exposed by running servers. Tool queries
are only allowed when a server is in the `running` lifecycle state.

```rust,no_run
use std::sync::Arc;

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

    let request = RegisterMcpServerRequest::new(
        "workspace_tools",
        McpTransport::stdio("mcp-server")?,
    );
    let registered = service.register(request).await?;
    let started = service.start(registered.id()).await?;
    assert_eq!(started.lifecycle_state().as_str(), "running");

    let servers = service.list_all().await?;
    assert_eq!(servers.len(), 1);

    let tools = service.list_tools(started.id()).await?;
    assert_eq!(tools.len(), 1);

    let stopped = service.stop(started.id()).await?;
    assert_eq!(stopped.lifecycle_state().as_str(), "stopped");

    let after_stop = service.list_tools(stopped.id()).await;
    assert!(after_stop.is_err());

    Ok(())
}
```
