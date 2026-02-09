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

/// Parameters for completing a handoff to a target agent in tests.
struct HandoffParams<'a> {
    target_agent: &'a str,
    start_sequence: SequenceNumber,
    reason: &'a str,
}

impl<'a> HandoffParams<'a> {
    const fn new(target_agent: &'a str, start_sequence: SequenceNumber, reason: &'a str) -> Self {
        Self {
            target_agent,
            start_sequence,
            reason,
        }
    }
}

/// Helper to initiate, create target session, and complete a handoff.
async fn complete_handoff_to_agent(
    harness: &HandoffTestHarness,
    source_session: &AgentSession,
    params: HandoffParams<'_>,
) -> TestResult<(corbusier::message::domain::HandoffMetadata, AgentSession)> {
    let initiate_params = ServiceInitiateParams::new(
        source_session.session_id,
        params.target_agent,
        TurnId::new(),
        params.start_sequence,
    )
    .with_reason(params.reason);

    let handoff = harness
        .service
        .initiate(initiate_params)
        .await
        .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;

    let session_params = HandoffSessionParams::new(
        source_session.conversation_id,
        params.target_agent,
        params.start_sequence,
        handoff.handoff_id,
    );

    let target_session = harness
        .service
        .create_target_session(session_params)
        .await
        .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;

    let completed = harness
        .service
        .complete(
            handoff.handoff_id,
            target_session.session_id,
            params.start_sequence,
        )
        .await
        .map_err(|err| Box::new(err) as Box<dyn std::error::Error + Send + Sync>)?;

    Ok((completed, target_session))
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
            &agent1,
            HandoffParams::new("agent-2", SequenceNumber::new(6), "escalate to specialist"),
        )
        .await
        .expect("handoff 1");
        let (_handoff2, _agent3) = complete_handoff_to_agent(
            &harness,
            &agent2,
            HandoffParams::new("agent-3", SequenceNumber::new(11), "need domain expert"),
        )
        .await
        .expect("handoff 2");

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
