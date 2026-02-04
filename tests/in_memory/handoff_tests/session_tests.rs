//! Session management tests for in-memory handoffs.

use super::harness::{HandoffTestHarness, TestResult, clock, harness, runtime};
use corbusier::message::domain::{
    AgentSession, AgentSessionState, ConversationId, HandoffSessionParams, SequenceNumber,
};
use corbusier::message::ports::agent_session::AgentSessionRepository;
use mockable::DefaultClock;
use rstest::rstest;
use tokio::runtime::Runtime;

#[rstest]
fn create_session_from_handoff_stores_correctly(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
) {
    let runtime_handle = runtime.expect("runtime");
    runtime_handle.block_on(async {
        let conversation_id = ConversationId::new();
        let handoff_id = corbusier::message::domain::HandoffId::new();

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
        assert_eq!(session.agent_backend, "target-agent");
        assert_eq!(session.initiated_by_handoff, Some(handoff_id));
        assert_eq!(session.state, AgentSessionState::Active);

        let found = harness
            .session_repo
            .find_by_id(session.session_id)
            .await
            .expect("should find")
            .expect("session should exist");

        assert_eq!(found.session_id, session.session_id);
    });
}

#[rstest]
fn session_repository_finds_active_session(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    let runtime_handle = runtime.expect("runtime");
    runtime_handle.block_on(async {
        let conversation_id = ConversationId::new();

        let session = AgentSession::new(conversation_id, "agent-1", SequenceNumber::new(1), &clock);

        harness
            .session_repo
            .store(&session)
            .await
            .expect("should store");

        let active = harness
            .session_repo
            .find_active_for_conversation(conversation_id)
            .await
            .expect("should query")
            .expect("should find active session");

        assert_eq!(active.session_id, session.session_id);
    });
}

#[rstest]
fn session_repository_lists_by_conversation(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    let runtime_handle = runtime.expect("runtime");
    runtime_handle.block_on(async {
        let conversation_id = ConversationId::new();

        let session1 =
            AgentSession::new(conversation_id, "agent-1", SequenceNumber::new(1), &clock);

        let session2 =
            AgentSession::new(conversation_id, "agent-2", SequenceNumber::new(10), &clock);

        harness
            .session_repo
            .store(&session1)
            .await
            .expect("store 1");
        harness
            .session_repo
            .store(&session2)
            .await
            .expect("store 2");

        let sessions = harness
            .session_repo
            .find_by_conversation(conversation_id)
            .await
            .expect("should list");

        assert_eq!(sessions.len(), 2);
    });
}
