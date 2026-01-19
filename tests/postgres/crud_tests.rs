//! Basic CRUD operation tests for `PostgreSQL` message repository.

use crate::postgres::cluster::BoxError;
use crate::postgres::helpers::{
    CleanupGuard, PostgresCluster, clock, create_test_message, ensure_template,
    insert_conversation, postgres_cluster, setup_repository, test_runtime,
};
use corbusier::message::adapters::postgres::PostgresMessageRepository;
use corbusier::message::{
    domain::{ConversationId, MessageId, Role},
    ports::repository::MessageRepository,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use tokio::runtime::Runtime;

struct CrudTestContext {
    cluster: PostgresCluster,
    db_name: String,
    guard: CleanupGuard<'static>,
    repo: PostgresMessageRepository,
    rt: Runtime,
}

fn test_error(message: impl Into<String>) -> BoxError {
    Box::new(std::io::Error::other(message.into()))
}

impl CrudTestContext {
    fn cleanup(self) -> Result<(), BoxError> {
        drop(self.repo);
        self.guard.cleanup()
    }
}

#[fixture]
fn crud_context(postgres_cluster: PostgresCluster) -> Result<CrudTestContext, BoxError> {
    let cluster = postgres_cluster;
    ensure_template(cluster)?;
    let db_name = format!("test_crud_{}", uuid::Uuid::new_v4());
    let guard = CleanupGuard::new(cluster, db_name.clone());
    let repo = setup_repository(cluster, &db_name)?;
    let rt = test_runtime()?;
    Ok(CrudTestContext {
        cluster,
        db_name,
        guard,
        repo,
        rt,
    })
}

#[rstest]
fn store_and_retrieve_message(
    clock: DefaultClock,
    crud_context: Result<CrudTestContext, BoxError>,
) -> Result<(), BoxError> {
    let context = crud_context?;

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)?;

    let message = create_test_message(&clock, conv_id, 1)?;
    let msg_id = message.id();

    context.rt.block_on(context.repo.store(&message))?;

    let retrieved = context
        .rt
        .block_on(context.repo.find_by_id(msg_id))?
        .expect("message should exist");

    if retrieved.id() != msg_id {
        return Err(test_error("retrieved message id does not match"));
    }
    if retrieved.conversation_id() != conv_id {
        return Err(test_error("retrieved conversation id does not match"));
    }
    if retrieved.role() != Role::User {
        return Err(test_error("retrieved role does not match"));
    }
    if retrieved.sequence_number().value() != 1 {
        return Err(test_error("retrieved sequence number does not match"));
    }

    context.cleanup()?;
    Ok(())
}

#[rstest]
fn find_by_id_returns_none_for_missing(
    crud_context: Result<CrudTestContext, BoxError>,
) -> Result<(), BoxError> {
    let context = crud_context?;

    let result = context
        .rt
        .block_on(context.repo.find_by_id(MessageId::new()))?;
    if result.is_some() {
        return Err(test_error("expected missing message to return None"));
    }

    context.cleanup()?;
    Ok(())
}

#[rstest]
fn find_by_conversation_returns_ordered_messages(
    clock: DefaultClock,
    crud_context: Result<CrudTestContext, BoxError>,
) -> Result<(), BoxError> {
    let context = crud_context?;

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)?;

    let msg3 = create_test_message(&clock, conv_id, 3)?;
    let msg1 = create_test_message(&clock, conv_id, 1)?;
    let msg2 = create_test_message(&clock, conv_id, 2)?;

    context.rt.block_on(context.repo.store(&msg3))?;
    context.rt.block_on(context.repo.store(&msg1))?;
    context.rt.block_on(context.repo.store(&msg2))?;

    let messages = context
        .rt
        .block_on(context.repo.find_by_conversation(conv_id))?;

    if messages.len() != 3 {
        return Err(test_error(format!(
            "expected 3 messages, got {}",
            messages.len()
        )));
    }
    let first = messages
        .first()
        .ok_or_else(|| test_error("missing first message"))?;
    let second = messages
        .get(1)
        .ok_or_else(|| test_error("missing second message"))?;
    let third = messages
        .get(2)
        .ok_or_else(|| test_error("missing third message"))?;
    if first.sequence_number().value() != 1 {
        return Err(test_error("first message sequence should be 1"));
    }
    if second.sequence_number().value() != 2 {
        return Err(test_error("second message sequence should be 2"));
    }
    if third.sequence_number().value() != 3 {
        return Err(test_error("third message sequence should be 3"));
    }

    context.cleanup()?;
    Ok(())
}

#[rstest]
fn exists_returns_correct_status(
    clock: DefaultClock,
    crud_context: Result<CrudTestContext, BoxError>,
) -> Result<(), BoxError> {
    let context = crud_context?;

    let conv_id = ConversationId::new();
    insert_conversation(context.cluster, &context.db_name, conv_id)?;

    let message = create_test_message(&clock, conv_id, 1)?;
    let msg_id = message.id();

    if context.rt.block_on(context.repo.exists(msg_id))? {
        return Err(test_error("expected missing message to return false"));
    }

    context.rt.block_on(context.repo.store(&message))?;
    if !context.rt.block_on(context.repo.exists(msg_id))? {
        return Err(test_error("expected existing message to return true"));
    }

    context.cleanup()?;
    Ok(())
}
