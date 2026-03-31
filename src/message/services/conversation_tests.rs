//! Unit tests for the conversation workflow service.

use super::{AppendMessageRequest, ConversationService, ConversationServiceError};
use crate::message::{
    adapters::memory::{InMemoryConversationRepository, InMemoryMessageRepository},
    domain::{ContentPart, ConversationId, Role, TextPart},
    validation::service::DefaultMessageValidator,
};
use crate::test_support::test_request_ctx;
use mockable::DefaultClock;
use rstest::rstest;
use std::sync::Arc;

type TestService = ConversationService<
    InMemoryConversationRepository,
    InMemoryMessageRepository,
    DefaultMessageValidator,
    DefaultClock,
>;

fn service() -> TestService {
    ConversationService::new(
        Arc::new(InMemoryConversationRepository::new()),
        Arc::new(InMemoryMessageRepository::new()),
        Arc::new(DefaultMessageValidator::new()),
        Arc::new(DefaultClock),
    )
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn create_and_read_history() -> Result<(), eyre::Report> {
    let service = service();
    let ctx = test_request_ctx();

    let conversation = service.create_conversation(&ctx).await?;
    let message = service
        .append_message(
            &ctx,
            AppendMessageRequest::new(
                conversation.id(),
                Role::User,
                vec![ContentPart::Text(TextPart::new("hello"))],
            ),
        )
        .await?;
    let history = service.history(&ctx, conversation.id()).await?;

    assert_eq!(history.len(), 1);
    assert_eq!(
        history
            .first()
            .expect("history should contain one message")
            .id(),
        message.id()
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn append_rejects_unknown_conversation() {
    let service = service();
    let ctx = test_request_ctx();

    let error = service
        .append_message(
            &ctx,
            AppendMessageRequest::new(
                ConversationId::new(),
                Role::User,
                vec![ContentPart::Text(TextPart::new("hello"))],
            ),
        )
        .await
        .expect_err("append should fail for missing conversation");

    assert!(matches!(
        error,
        ConversationServiceError::ConversationNotFound(_)
    ));
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn append_validates_content() {
    let service = service();
    let ctx = test_request_ctx();
    let conversation = service
        .create_conversation(&ctx)
        .await
        .expect("conversation should be created");

    let error = service
        .append_message(
            &ctx,
            AppendMessageRequest::new(
                conversation.id(),
                Role::User,
                vec![ContentPart::Text(TextPart::new("   "))],
            ),
        )
        .await
        .expect_err("empty text should be rejected");

    assert!(matches!(
        error,
        ConversationServiceError::Validation(validation)
            if !validation.to_string().is_empty()
    ));
}
