//! Tests for the handoff service.

use super::{HandoffService, ServiceInitiateParams};
use crate::message::{
    adapters::memory::{
        InMemoryAgentSessionRepository, InMemoryContextSnapshotAdapter, InMemoryHandoffAdapter,
    },
    domain::{
        AgentSessionId, ConversationId, HandoffId, HandoffSessionParams, SequenceNumber, TurnId,
    },
    ports::agent_session::AgentSessionRepository,
    ports::handoff::HandoffError,
};
use std::sync::Arc;

struct ServiceHarness {
    service: HandoffService<
        InMemoryAgentSessionRepository,
        InMemoryHandoffAdapter<mockable::DefaultClock>,
        InMemoryContextSnapshotAdapter,
        mockable::DefaultClock,
    >,
    session_repo: Arc<InMemoryAgentSessionRepository>,
}

fn create_service() -> ServiceHarness {
    let session_repo = Arc::new(InMemoryAgentSessionRepository::new());
    let clock = Arc::new(mockable::DefaultClock);
    let handoff_adapter = Arc::new(InMemoryHandoffAdapter::new(mockable::DefaultClock));
    let snapshot_adapter = Arc::new(InMemoryContextSnapshotAdapter::new());

    let service = HandoffService::new(
        Arc::clone(&session_repo),
        handoff_adapter,
        snapshot_adapter,
        clock,
    );

    ServiceHarness {
        service,
        session_repo,
    }
}

#[tokio::test]
async fn initiate_handoff_requires_active_session() {
    let service = create_service().service;
    let conversation_id = ConversationId::new();
    let session_id = AgentSessionId::new();

    let params = ServiceInitiateParams::new(
        conversation_id,
        session_id,
        "target-agent",
        TurnId::new(),
        SequenceNumber::new(5),
    );
    let result = service.initiate(params).await;

    assert!(result.is_err());
    let err = result.expect_err("should be error");
    assert!(matches!(err, HandoffError::SessionNotFound(_)));
}

#[tokio::test]
async fn create_target_session_stores_session() {
    let harness = create_service();
    let conversation_id = ConversationId::new();
    let handoff_id = HandoffId::new();

    let params = HandoffSessionParams::new(
        conversation_id,
        "target-agent",
        SequenceNumber::new(10),
        handoff_id,
    );
    let session = harness
        .service
        .create_target_session(params)
        .await
        .expect("should create session");

    assert_eq!(session.conversation_id, conversation_id);
    assert_eq!(session.initiated_by_handoff, Some(handoff_id));
    assert_eq!(session.agent_backend, "target-agent");

    let found = harness
        .session_repo
        .find_by_id(session.session_id)
        .await
        .expect("should find")
        .expect("session should exist");

    assert_eq!(found.session_id, session.session_id);
}
