//! Shared fixtures and helpers for validation tests.

use crate::message::{
    domain::{ContentPart, ConversationId, Message, Role, SequenceNumber},
    ports::validator::ValidationConfig,
    validation::service::DefaultMessageValidator,
};
use mockable::DefaultClock;
use rstest::fixture;

#[fixture]
pub fn default_validator() -> DefaultMessageValidator {
    DefaultMessageValidator::new()
}

#[fixture]
pub fn lenient_validator() -> DefaultMessageValidator {
    DefaultMessageValidator::with_config(ValidationConfig::lenient())
}

#[fixture]
pub fn strict_validator() -> DefaultMessageValidator {
    DefaultMessageValidator::with_config(ValidationConfig::strict())
}

#[fixture]
pub fn clock() -> DefaultClock {
    DefaultClock
}

/// Factory fixture for creating test messages with a given role and content.
#[fixture]
pub fn message_factory(clock: DefaultClock) -> impl Fn(Role, Vec<ContentPart>) -> Message {
    move |role, content| {
        Message::new(
            ConversationId::new(),
            role,
            content,
            SequenceNumber::new(1),
            &clock,
        )
        .expect("test message should build")
    }
}

/// Helper function for creating test messages (legacy compatibility).
pub fn create_message(role: Role, content: Vec<ContentPart>, clock: &DefaultClock) -> Message {
    Message::new(
        ConversationId::new(),
        role,
        content,
        SequenceNumber::new(1),
        clock,
    )
    .expect("test message should build")
}
