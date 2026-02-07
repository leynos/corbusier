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

/// Helper to initiate, create target session, and complete a handoff.
#[expect(
    clippy::too_many_arguments,
    reason = "Test helper mirrors the full handoff flow inputs"
)]
async fn complete_handoff_to_agent(
    harness: &HandoffTestHarness,
    conversation_id: ConversationId,
    source_session: &AgentSession,
    target_agent: &str,
    start_sequence: SequenceNumber,
    reason: &str,
) -> (corbusier::message::domain::HandoffMetadata, AgentSession) {
    let initiate_params = ServiceInitiateParams::new(
        source_session.session_id,
        target_agent,
        TurnId::new(),
        start_sequence,
    )
    .with_reason(reason);

    let handoff = harness
        .service
        .initiate(initiate_params)
        .await
        .unwrap_or_else(|_err| panic!("initiate handoff"));

    let session_params = HandoffSessionParams::new(
        conversation_id,
        target_agent,
        start_sequence,
        handoff.handoff_id,
    );

    let target_session = harness
        .service
        .create_target_session(session_params)
        .await
        .unwrap_or_else(|_err| panic!("create target session"));

    let completed = harness
        .service
        .complete(
            handoff.handoff_id,
            target_session.session_id,
            start_sequence,
        )
        .await
        .unwrap_or_else(|_err| panic!("complete handoff"));

    (completed, target_session)
}

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

        let (_handoff1, agent2) = complete_handoff_to_agent(
            &harness,
            conversation_id,
            &agent1,
            "agent-2",
            SequenceNumber::new(6),
            "escalate to specialist",
        )
        .await;
        let (_handoff2, _agent3) = complete_handoff_to_agent(
            &harness,
            conversation_id,
            &agent2,
            "agent-3",
            SequenceNumber::new(11),
            "need domain expert",
        )
        .await;

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
