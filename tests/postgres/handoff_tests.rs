//! Integration tests for PostgreSQL handoff persistence.

use crate::postgres::cluster::BoxError;
use crate::postgres::helpers::{
    PreparedRepo, build_pool, clock, insert_conversation, prepared_repo, test_request_context,
};
use corbusier::context::RequestContext;
use corbusier::message::{
    adapters::postgres::{PostgresAgentSessionRepository, PostgresHandoffAdapter},
    domain::{AgentSession, ConversationId, SequenceNumber, TurnId},
    ports::{
        agent_session::AgentSessionRepository,
        handoff::{AgentHandoffPort, InitiateHandoffParams},
    },
};
use mockable::DefaultClock;
use rstest::rstest;

fn missing_handoff_error() -> BoxError {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "expected handoff to exist",
    ))
}

#[rstest]
#[tokio::test]
async fn initiate_and_list_handoffs_for_conversation(
    clock: DefaultClock,
    test_request_context: RequestContext,
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let prep = prepared_repo.await?;
    let pool = build_pool(prep.temp_db.url(), 1)?;

    let session_repo = PostgresAgentSessionRepository::new(pool.clone());
    let handoff_adapter = PostgresHandoffAdapter::new(pool);

    let ctx = test_request_context;
    let conversation_id = ConversationId::new();
    insert_conversation(prep.cluster, prep.temp_db.name(), conversation_id).await?;

    let source_session = AgentSession::new(
        conversation_id,
        "agent-a",
        SequenceNumber::new(1),
        &clock,
    );
    session_repo.store(&ctx, &source_session).await?;

    let params = InitiateHandoffParams::new(
        conversation_id,
        &source_session,
        "agent-b",
        TurnId::new(),
    )
    .with_reason("escalation");
    let handoff = handoff_adapter.initiate_handoff(&ctx, params).await?;

    let handoffs = handoff_adapter
        .list_handoffs_for_conversation(&ctx, conversation_id)
        .await?;

    assert_eq!(handoffs.len(), 1);
    let first = handoffs
        .first()
        .ok_or_else(missing_handoff_error)?;
    assert_eq!(first.handoff_id, handoff.handoff_id);
    Ok(())
}

#[rstest]
#[tokio::test]
async fn complete_handoff_updates_target_and_status(
    clock: DefaultClock,
    test_request_context: RequestContext,
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let prep = prepared_repo.await?;
    let pool = build_pool(prep.temp_db.url(), 1)?;

    let session_repo = PostgresAgentSessionRepository::new(pool.clone());
    let handoff_adapter = PostgresHandoffAdapter::new(pool);

    let ctx = test_request_context;
    let conversation_id = ConversationId::new();
    insert_conversation(prep.cluster, prep.temp_db.name(), conversation_id).await?;

    let source_session = AgentSession::new(
        conversation_id,
        "agent-a",
        SequenceNumber::new(1),
        &clock,
    );
    session_repo.store(&ctx, &source_session).await?;

    let params =
        InitiateHandoffParams::new(conversation_id, &source_session, "agent-b", TurnId::new());
    let handoff = handoff_adapter.initiate_handoff(&ctx, params).await?;

    let target_session = AgentSession::new(
        conversation_id,
        "agent-b",
        SequenceNumber::new(10),
        &clock,
    );
    session_repo.store(&ctx, &target_session).await?;

    let completed = handoff_adapter
        .complete_handoff(&ctx, handoff.handoff_id, target_session.session_id)
        .await?;

    assert_eq!(completed.target_session_id, Some(target_session.session_id));
    assert!(completed.completed_at.is_some());
    Ok(())
}

#[rstest]
#[tokio::test]
async fn cancel_handoff_persists_reason(
    clock: DefaultClock,
    test_request_context: RequestContext,
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let prep = prepared_repo.await?;
    let pool = build_pool(prep.temp_db.url(), 1)?;

    let session_repo = PostgresAgentSessionRepository::new(pool.clone());
    let handoff_adapter = PostgresHandoffAdapter::new(pool);

    let ctx = test_request_context;
    let conversation_id = ConversationId::new();
    insert_conversation(prep.cluster, prep.temp_db.name(), conversation_id).await?;

    let source_session = AgentSession::new(
        conversation_id,
        "agent-a",
        SequenceNumber::new(1),
        &clock,
    );
    session_repo.store(&ctx, &source_session).await?;

    let params =
        InitiateHandoffParams::new(conversation_id, &source_session, "agent-b", TurnId::new());
    let handoff = handoff_adapter.initiate_handoff(&ctx, params).await?;

    handoff_adapter
        .cancel_handoff(&ctx, handoff.handoff_id, Some("target unavailable"))
        .await?;

    let found = handoff_adapter
        .find_handoff(&ctx, handoff.handoff_id)
        .await?
        .ok_or_else(missing_handoff_error)?;

    assert_eq!(found.status.as_str(), "cancelled");
    assert_eq!(found.reason, Some("target unavailable".to_owned()));
    Ok(())
}

/// Cancelling with `None` reason preserves the original reason set at initiation.
#[rstest]
#[tokio::test]
async fn cancel_handoff_with_none_preserves_original_reason(
    clock: DefaultClock,
    test_request_context: RequestContext,
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let prep = prepared_repo.await?;
    let pool = build_pool(prep.temp_db.url(), 1)?;

    let session_repo = PostgresAgentSessionRepository::new(pool.clone());
    let handoff_adapter = PostgresHandoffAdapter::new(pool);

    let ctx = test_request_context;
    let conversation_id = ConversationId::new();
    insert_conversation(prep.cluster, prep.temp_db.name(), conversation_id).await?;

    let source_session = AgentSession::new(
        conversation_id,
        "agent-a",
        SequenceNumber::new(1),
        &clock,
    );
    session_repo.store(&ctx, &source_session).await?;

    let params = InitiateHandoffParams::new(
        conversation_id,
        &source_session,
        "agent-b",
        TurnId::new(),
    )
    .with_reason("escalation needed");
    let handoff = handoff_adapter.initiate_handoff(&ctx, params).await?;

    // Cancel with None — the original reason should be preserved.
    handoff_adapter
        .cancel_handoff(&ctx, handoff.handoff_id, None)
        .await?;

    let found = handoff_adapter
        .find_handoff(&ctx, handoff.handoff_id)
        .await?
        .ok_or_else(missing_handoff_error)?;

    assert_eq!(found.status.as_str(), "cancelled");
    assert_eq!(
        found.reason,
        Some("escalation needed".to_owned()),
        "cancelling with None should preserve the original initiation reason"
    );
    Ok(())
}
