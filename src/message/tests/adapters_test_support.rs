//! Shared fixtures and helpers for message adapter tests.

use crate::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use crate::message::{
    adapters::memory::InMemoryMessageRepository,
    domain::{
        ContentPart, ConversationId, Message, MessageBuilderError, Role, SequenceNumber, TextPart,
    },
};
use mockable::DefaultClock;
use rstest::fixture;

#[fixture]
pub(super) fn ctx() -> RequestContext {
    RequestContext::new(
        TenantId::new(),
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    )
}

#[fixture]
pub(super) fn clock() -> DefaultClock {
    DefaultClock
}

#[fixture]
pub(super) fn repo() -> InMemoryMessageRepository {
    InMemoryMessageRepository::new()
}

pub(super) fn make_message(
    conversation_id: ConversationId,
    seq: u64,
    clock: &DefaultClock,
) -> Result<Message, MessageBuilderError> {
    Message::new(
        conversation_id,
        Role::User,
        vec![ContentPart::Text(TextPart::new(format!("Message {seq}")))],
        SequenceNumber::new(seq),
        clock,
    )
}
