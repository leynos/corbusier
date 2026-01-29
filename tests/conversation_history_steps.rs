//! BDD steps for conversation history persistence with audit metadata.

use corbusier::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{
        AgentResponseAudit, AgentResponseStatus, ContentPart, ConversationId, Message,
        MessageMetadata, Role, SequenceNumber, TextPart, ToolCallAudit, ToolCallStatus,
    },
    error::ValidationError,
    ports::repository::MessageRepository,
    ports::validator::MessageValidator,
    validation::service::DefaultMessageValidator,
};
use mockable::DefaultClock;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

#[derive(Default)]
struct HistoryWorld {
    repo: InMemoryMessageRepository,
    conversation_id: ConversationId,
    last_validation_error: Option<ValidationError>,
}

#[fixture]
fn world() -> HistoryWorld {
    HistoryWorld::default()
}

fn run_async<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
}

fn assert_metadata_error(error: &ValidationError) {
    match error {
        ValidationError::InvalidMetadata(_) => {}
        ValidationError::Multiple(errors) => {
            assert!(
                errors
                    .iter()
                    .any(|err| matches!(err, ValidationError::InvalidMetadata(_))),
                "expected metadata validation error"
            );
        }
        _ => panic!("expected metadata validation error, got {error:?}"),
    }
}

#[given("an empty conversation history")]
fn empty_conversation_history(world: &mut HistoryWorld) {
    *world = HistoryWorld::default();
}

#[when("a tool call and agent response are persisted")]
fn persist_tool_call_and_agent_response(world: &mut HistoryWorld) {
    let clock = DefaultClock;
    let metadata = MessageMetadata::empty()
        .with_tool_call_audit(ToolCallAudit::new(
            "call-123",
            "read_file",
            ToolCallStatus::Succeeded,
        ))
        .with_agent_response_audit(
            AgentResponseAudit::new(AgentResponseStatus::Completed).with_response_id("resp-456"),
        );
    let message = match Message::builder(
        world.conversation_id,
        Role::Assistant,
        SequenceNumber::new(1),
    )
    .with_content(ContentPart::Text(TextPart::new("Response")))
    .with_metadata(metadata)
    .build(&clock)
    {
        Ok(message) => message,
        Err(error) => panic!("message should build: {error}"),
    };

    if let Err(error) = run_async(world.repo.store(&message)) {
        panic!("store should succeed: {error}");
    }
}

#[then("the conversation history includes audit metadata")]
fn history_includes_audit_metadata(world: &HistoryWorld) {
    let history = match run_async(world.repo.find_by_conversation(world.conversation_id)) {
        Ok(history) => history,
        Err(error) => panic!("history fetch should succeed: {error}"),
    };

    let Some(message) = history.first() else {
        panic!("expected at least one message");
    };
    assert_eq!(message.metadata().tool_call_audits.len(), 1);
    assert!(message.metadata().agent_response_audit.is_some());
}

#[when("a tool call audit is missing a call id")]
fn tool_call_audit_missing_call_id(world: &mut HistoryWorld) {
    let clock = DefaultClock;
    let metadata = MessageMetadata::empty().with_tool_call_audit(ToolCallAudit::new(
        "",
        "read_file",
        ToolCallStatus::Queued,
    ));
    let message = match Message::builder(
        world.conversation_id,
        Role::Assistant,
        SequenceNumber::new(1),
    )
    .with_content(ContentPart::Text(TextPart::new("Response")))
    .with_metadata(metadata)
    .build(&clock)
    {
        Ok(message) => message,
        Err(error) => panic!("message should build: {error}"),
    };

    let validator = DefaultMessageValidator::new();
    world.last_validation_error = validator.validate_structure(&message).err();
}

#[then("the message is rejected with a metadata error")]
fn message_rejected_with_metadata_error(world: &HistoryWorld) {
    let Some(error) = world.last_validation_error.as_ref() else {
        panic!("expected a validation error");
    };
    assert_metadata_error(error);
}

#[scenario(
    path = "tests/features/conversation_history.feature",
    name = "Persist tool call and agent response audit metadata"
)]
#[tokio::test(flavor = "multi_thread")]
async fn persist_audit_metadata(world: HistoryWorld) {
    // World parameter required for rstest-bdd fixture injection; step
    // definitions handle mutation.
    let _ = world;
}

#[scenario(
    path = "tests/features/conversation_history.feature",
    name = "Missing tool call audit metadata is rejected"
)]
#[tokio::test(flavor = "multi_thread")]
async fn reject_missing_tool_call_audit(world: HistoryWorld) {
    // World parameter required for rstest-bdd fixture injection; step
    // definitions handle mutation.
    let _ = world;
}
