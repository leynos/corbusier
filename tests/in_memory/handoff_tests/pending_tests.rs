//! Pending handoff query tests for in-memory adapters.

use super::harness::{HandoffTestHarness, TestResult, clock, harness, runtime};
use corbusier::message::domain::{
    AgentSession, ConversationId, HandoffSessionParams, SequenceNumber, TurnId,
};
use corbusier::message::ports::agent_session::AgentSessionRepository;
use corbusier::message::services::ServiceInitiateParams;
use mockable::DefaultClock;
use rstest::rstest;
use tokio::runtime::Runtime;

#[rstest]
fn get_pending_handoff_returns_initiated(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    let runtime_handle = runtime.expect("runtime");
    runtime_handle.block_on(async {
        let conversation_id = ConversationId::new();

        let source_session = AgentSession::new(
            conversation_id,
            "source-agent",
            SequenceNumber::new(1),
            &clock,
        );

        harness
            .session_repo
            .store(&source_session)
            .await
            .expect("store");

        let initiate_params = ServiceInitiateParams::new(
            conversation_id,
            source_session.session_id,
            "target-agent",
            TurnId::new(),
            SequenceNumber::new(5),
        );
        let handoff = harness
            .service
            .initiate(initiate_params)
            .await
            .expect("initiate");

        let pending = harness
            .service
            .get_pending_handoff(conversation_id)
            .await
            .expect("query")
            .expect("should have pending");

        assert_eq!(pending.handoff_id, handoff.handoff_id);
    });
}

#[rstest]
fn get_pending_handoff_returns_none_when_completed(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    let runtime_handle = runtime.expect("runtime");
    runtime_handle.block_on(async {
        let conversation_id = ConversationId::new();

        let source_session = AgentSession::new(
            conversation_id,
            "source-agent",
            SequenceNumber::new(1),
            &clock,
        );

        harness
            .session_repo
            .store(&source_session)
            .await
            .expect("store");

        let initiate_params = ServiceInitiateParams::new(
            conversation_id,
            source_session.session_id,
            "target-agent",
            TurnId::new(),
            SequenceNumber::new(5),
        );
        let handoff = harness
            .service
            .initiate(initiate_params)
            .await
            .expect("initiate");

        let session_params = HandoffSessionParams::new(
            conversation_id,
            "target-agent",
            SequenceNumber::new(6),
            handoff.handoff_id,
        );
        let target = harness
            .service
            .create_target_session(session_params)
            .await
            .expect("target");

        harness
            .service
            .complete(
                handoff.handoff_id,
                target.session_id,
                SequenceNumber::new(6),
            )
            .await
            .expect("complete");

        let pending = harness
            .service
            .get_pending_handoff(conversation_id)
            .await
            .expect("query");

        assert!(pending.is_none());
    });
}
