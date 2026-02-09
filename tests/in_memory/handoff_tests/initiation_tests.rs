//! Handoff initiation tests for in-memory adapters.

use super::harness::{HandoffTestHarness, TestResult, clock, harness, runtime};
use corbusier::message::domain::{
    AgentSession, AgentSessionState, ConversationId, HandoffStatus, SequenceNumber, TurnId,
};
use corbusier::message::ports::agent_session::AgentSessionRepository;
use corbusier::message::services::ServiceInitiateParams;
use mockable::DefaultClock;
use rstest::rstest;
use tokio::runtime::Runtime;

#[rstest]
fn initiate_handoff_requires_active_session(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
) {
    let runtime_handle = runtime.expect("runtime");
    runtime_handle.block_on(async {
        let session_id = corbusier::message::domain::AgentSessionId::new();

        let params = ServiceInitiateParams::new(
            session_id,
            "target-agent",
            TurnId::new(),
            SequenceNumber::new(5),
        )
        .with_reason("task too complex");
        let result = harness.service.initiate(params).await;

        assert!(result.is_err());
    });
}

#[rstest]
fn initiate_handoff_succeeds_with_active_session(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    let runtime_handle = runtime.expect("runtime");
    runtime_handle.block_on(async {
        let conversation_id = ConversationId::new();

        let session = AgentSession::new(
            conversation_id,
            "source-agent",
            SequenceNumber::new(1),
            &clock,
        );

        harness.session_repo.store(&session).await.expect("store");

        let initiate_params = ServiceInitiateParams::new(
            session.session_id,
            "target-agent",
            TurnId::new(),
            SequenceNumber::new(5),
        )
        .with_reason("task requires specialist");
        let handoff = harness
            .service
            .initiate(initiate_params)
            .await
            .expect("should initiate");

        assert_eq!(handoff.source_session_id, session.session_id);
        assert_eq!(handoff.source_agent, "source-agent");
        assert_eq!(handoff.target_agent, "target-agent");
        assert_eq!(handoff.status, HandoffStatus::Initiated);
        assert_eq!(handoff.reason, Some("task requires specialist".to_owned()));

        let updated = harness
            .session_repo
            .find_by_id(session.session_id)
            .await
            .expect("find")
            .expect("exists");

        assert_eq!(updated.state, AgentSessionState::HandedOff);
        assert_eq!(updated.terminated_by_handoff, Some(handoff.handoff_id));
    });
}
