//! Handoff completion tests for in-memory adapters.

use super::harness::{HandoffTestHarness, TestResult, clock, harness, runtime};
use corbusier::message::domain::{
    AgentSession, ConversationId, HandoffSessionParams, HandoffStatus, SequenceNumber, TurnId,
};
use corbusier::message::ports::agent_session::AgentSessionRepository;
use corbusier::message::services::ServiceInitiateParams;
use mockable::DefaultClock;
use rstest::rstest;
use tokio::runtime::Runtime;

#[rstest]
fn complete_handoff_links_target_session(
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

        let params = HandoffSessionParams::new(
            conversation_id,
            "target-agent",
            SequenceNumber::new(6),
            handoff.handoff_id,
        );
        let target_session = harness
            .service
            .create_target_session(params)
            .await
            .expect("create target");

        let completed = harness
            .service
            .complete(
                handoff.handoff_id,
                target_session.session_id,
                SequenceNumber::new(6),
            )
            .await
            .expect("complete");

        assert_eq!(completed.status, HandoffStatus::Completed);
        assert_eq!(completed.target_session_id, Some(target_session.session_id));
        assert!(completed.completed_at.is_some());
    });
}
