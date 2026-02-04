//! Handoff cancellation tests for in-memory adapters.

use super::harness::{HandoffTestHarness, TestResult, clock, harness, runtime};
use corbusier::message::domain::{
    AgentSession, AgentSessionState, ConversationId, SequenceNumber, TurnId,
};
use corbusier::message::ports::{agent_session::AgentSessionRepository, handoff::AgentHandoffPort};
use corbusier::message::services::ServiceInitiateParams;
use mockable::DefaultClock;
use rstest::rstest;
use tokio::runtime::Runtime;

#[rstest]
fn cancel_handoff_reverts_source_session(
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

        let reason = "target agent unavailable";
        harness
            .service
            .cancel(handoff.handoff_id, Some(reason))
            .await
            .expect("cancel");

        let reverted = harness
            .session_repo
            .find_by_id(source_session.session_id)
            .await
            .expect("find")
            .expect("exists");

        assert_eq!(reverted.state, AgentSessionState::Active);
        assert_eq!(reverted.terminated_by_handoff, None);

        let stored = harness
            .handoff_adapter
            .find_handoff(handoff.handoff_id)
            .await
            .expect("find handoff")
            .expect("handoff exists");

        assert_eq!(stored.reason, Some(reason.to_owned()));
    });
}
