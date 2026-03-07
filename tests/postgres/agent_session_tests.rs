//! Integration tests for `PostgreSQL` agent session persistence.
//!
//! Covers the unique active-session-per-conversation constraint, verifying
//! that the partial unique index `idx_agent_sessions_one_active_per_conversation`
//! prevents TOCTOU races when two active sessions are stored concurrently.

use crate::postgres::cluster::BoxError;
use crate::postgres::helpers::{
    PreparedRepo, build_pool, insert_conversation, prepared_repo, test_request_context,
};
use corbusier::context::{CorrelationId, RequestContext, SessionId, TenantId, UserId};
use corbusier::message::{
    adapters::postgres::PostgresAgentSessionRepository,
    domain::{AgentSession, ConversationId, SequenceNumber},
    ports::agent_session::{AgentSessionRepository, SessionError},
};
use mockable::DefaultClock;
use rstest::rstest;

/// Proves that two concurrent `store` calls for active sessions on the same
/// conversation produce exactly one `ActiveSessionExists` error and one
/// success.
///
/// This test exercises the partial unique index
/// `idx_agent_sessions_one_active_per_conversation` which closes the TOCTOU
/// race between the insert and the `check_no_active_session` query.
#[rstest]
#[tokio::test]
async fn concurrent_active_session_store_rejects_second(
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let prep = prepared_repo.await?;
    // Two connections so both tasks can run their transactions concurrently.
    let pool = build_pool(prep.temp_db.url(), 2)?;
    let repo = PostgresAgentSessionRepository::new(pool);

    let tenant_id = TenantId::new();
    let ctx = RequestContext::new(
        tenant_id,
        CorrelationId::new(),
        UserId::new(),
        SessionId::new(),
    );
    let conversation_id = ConversationId::new();
    insert_conversation(prep.cluster, prep.temp_db.name(), conversation_id).await?;

    let clock = DefaultClock;
    let session_a = AgentSession::new(conversation_id, "agent-a", SequenceNumber::new(1), &clock);
    let session_b = AgentSession::new(conversation_id, "agent-b", SequenceNumber::new(2), &clock);

    // Launch both stores concurrently.
    let repo_a = repo.clone();
    let ctx_a = ctx.clone();
    let handle_a = tokio::spawn(async move { repo_a.store(&ctx_a, &session_a).await });

    let repo_b = repo.clone();
    let ctx_b = ctx.clone();
    let handle_b = tokio::spawn(async move { repo_b.store(&ctx_b, &session_b).await });

    let result_a = handle_a.await.map_err(|e| Box::new(e) as BoxError)?;
    let result_b = handle_b.await.map_err(|e| Box::new(e) as BoxError)?;

    // Exactly one should succeed and one should fail with ActiveSessionExists.
    let (successes, failures): (Vec<_>, Vec<_>) =
        [result_a, result_b].into_iter().partition(Result::is_ok);

    assert_eq!(
        successes.len(),
        1,
        "exactly one store should succeed; got {successes:?}"
    );
    assert_eq!(
        failures.len(),
        1,
        "exactly one store should fail; got {failures:?}"
    );

    let err = failures
        .into_iter()
        .next()
        .expect("already asserted exactly one failure")
        .expect_err("partition guarantees this is Err");
    assert!(
        matches!(err, SessionError::ActiveSessionExists(cid) if cid == conversation_id),
        "expected ActiveSessionExists({conversation_id}), got {err:?}"
    );

    Ok(())
}

/// Verifies that a sequential second `store` of an active session on the same
/// conversation returns `ActiveSessionExists`.
#[rstest]
#[tokio::test]
async fn sequential_duplicate_active_session_store_is_rejected(
    test_request_context: RequestContext,
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let prep = prepared_repo.await?;
    let pool = build_pool(prep.temp_db.url(), 1)?;
    let repo = PostgresAgentSessionRepository::new(pool);

    let ctx = test_request_context;
    let conversation_id = ConversationId::new();
    insert_conversation(prep.cluster, prep.temp_db.name(), conversation_id).await?;

    let clock = DefaultClock;
    let session_a = AgentSession::new(conversation_id, "agent-a", SequenceNumber::new(1), &clock);
    let session_b = AgentSession::new(conversation_id, "agent-b", SequenceNumber::new(2), &clock);

    repo.store(&ctx, &session_a).await?;

    let err = repo
        .store(&ctx, &session_b)
        .await
        .expect_err("second active session should be rejected");

    assert!(
        matches!(err, SessionError::ActiveSessionExists(cid) if cid == conversation_id),
        "expected ActiveSessionExists({conversation_id}), got {err:?}"
    );

    Ok(())
}

/// Verifies that updating a session to active when another active session
/// already exists returns `ActiveSessionExists`.
#[rstest]
#[tokio::test]
async fn update_to_active_when_another_active_exists_is_rejected(
    test_request_context: RequestContext,
    #[future] prepared_repo: Result<PreparedRepo, BoxError>,
) -> Result<(), BoxError> {
    let prep = prepared_repo.await?;
    let pool = build_pool(prep.temp_db.url(), 1)?;
    let repo = PostgresAgentSessionRepository::new(pool);

    let ctx = test_request_context;
    let conversation_id = ConversationId::new();
    insert_conversation(prep.cluster, prep.temp_db.name(), conversation_id).await?;

    let clock = DefaultClock;

    // Store an active session.
    let session_a = AgentSession::new(conversation_id, "agent-a", SequenceNumber::new(1), &clock);
    repo.store(&ctx, &session_a).await?;

    // Store a second session in paused state (does not conflict).
    let mut session_b =
        AgentSession::new(conversation_id, "agent-b", SequenceNumber::new(2), &clock);
    session_b.pause();
    repo.store(&ctx, &session_b).await?;

    // Now try to update session_b to active — should fail.
    session_b.resume();
    let err = repo
        .update(&ctx, &session_b)
        .await
        .expect_err("activating second session should be rejected");

    assert!(
        matches!(err, SessionError::ActiveSessionExists(cid) if cid == conversation_id),
        "expected ActiveSessionExists({conversation_id}), got {err:?}"
    );

    Ok(())
}
