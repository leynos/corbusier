//! Multi-handoff chain tests for in-memory adapters.

use super::harness::{HandoffTestHarness, TestResult, clock, harness, runtime};
use corbusier::message::domain::{
    AgentSession, ConversationId, HandoffSessionParams, HandoffStatus, SequenceNumber, TurnId,
};
use corbusier::message::ports::{agent_session::AgentSessionRepository, handoff::AgentHandoffPort};
use corbusier::message::services::ServiceInitiateParams;
use mockable::DefaultClock;
use rstest::rstest;
use tokio::runtime::Runtime;

#[rstest]
fn handoff_chain_tracks_all_sessions(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    let runtime_handle = runtime.expect("runtime");
    runtime_handle.block_on(async {
        let conversation_id = ConversationId::new();

        let agent1 = AgentSession::new(conversation_id, "agent-1", SequenceNumber::new(1), &clock);
        harness.session_repo.store(&agent1).await.expect("store 1");

        let initiate1 = ServiceInitiateParams::new(
            conversation_id,
            agent1.session_id,
            "agent-2",
            TurnId::new(),
            SequenceNumber::new(5),
        )
        .with_reason("escalate to specialist");
        let handoff1 = harness
            .service
            .initiate(initiate1)
            .await
            .expect("initiate 1");

        let params2 = HandoffSessionParams::new(
            conversation_id,
            "agent-2",
            SequenceNumber::new(6),
            handoff1.handoff_id,
        );
        let agent2 = harness
            .service
            .create_target_session(params2)
            .await
            .expect("create agent2");

        harness
            .service
            .complete(
                handoff1.handoff_id,
                agent2.session_id,
                SequenceNumber::new(6),
            )
            .await
            .expect("complete 1");

        let initiate2 = ServiceInitiateParams::new(
            conversation_id,
            agent2.session_id,
            "agent-3",
            TurnId::new(),
            SequenceNumber::new(10),
        )
        .with_reason("need domain expert");
        let handoff2 = harness
            .service
            .initiate(initiate2)
            .await
            .expect("initiate 2");

        let params3 = HandoffSessionParams::new(
            conversation_id,
            "agent-3",
            SequenceNumber::new(11),
            handoff2.handoff_id,
        );
        let agent3 = harness
            .service
            .create_target_session(params3)
            .await
            .expect("create agent3");

        harness
            .service
            .complete(
                handoff2.handoff_id,
                agent3.session_id,
                SequenceNumber::new(11),
            )
            .await
            .expect("complete 2");

        let sessions = harness
            .session_repo
            .find_by_conversation(conversation_id)
            .await
            .expect("list");

        assert_eq!(sessions.len(), 3);

        let handoffs = harness
            .handoff_adapter
            .list_handoffs_for_conversation(conversation_id)
            .await
            .expect("list handoffs");

        assert_eq!(handoffs.len(), 2);
        assert!(
            handoffs
                .iter()
                .all(|h| h.status == HandoffStatus::Completed)
        );
    });
}
