//! Context snapshot tests for in-memory handoff flows.

use super::harness::{HandoffTestHarness, TestResult, clock, ctx, harness, runtime};
use corbusier::context::RequestContext;
use corbusier::message::domain::{AgentSession, ConversationId, SequenceNumber, TurnId};
use corbusier::message::ports::{
    agent_session::AgentSessionRepository, context_snapshot::ContextSnapshotPort,
};
use corbusier::message::services::ServiceInitiateParams;
use mockable::DefaultClock;
use rstest::rstest;
use tokio::runtime::Runtime;

#[rstest]
fn handoff_captures_context_snapshot(
    runtime: TestResult<Runtime>,
    harness: HandoffTestHarness,
    ctx: RequestContext,
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
            .store(&ctx, &source_session)
            .await
            .expect("store");

        let initiate_params = ServiceInitiateParams::new(
            source_session.session_id,
            "target-agent",
            TurnId::new(),
            SequenceNumber::new(5),
        );
        let _handoff = harness
            .service
            .initiate(&ctx, initiate_params)
            .await
            .expect("initiate");

        let snapshots = harness
            .snapshot_adapter
            .find_snapshots_for_session(&ctx, source_session.session_id)
            .await
            .expect("find snapshots");

        let snapshot = snapshots.first().expect("snapshot should exist");
        assert_eq!(
            snapshot.snapshot_type,
            corbusier::message::domain::SnapshotType::HandoffInitiated
        );
    });
}
