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
