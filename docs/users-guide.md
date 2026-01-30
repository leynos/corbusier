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
