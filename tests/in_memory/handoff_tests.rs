//! Integration tests for handoff operations using in-memory adapters.
//!
//! Tests the complete handoff workflow including:
//! - Session creation and management
//! - Handoff initiation and completion
//! - Context snapshot capture
//! - Error handling for invalid states

use corbusier::message::{
    adapters::memory::{
        InMemoryAgentSessionRepository, InMemoryContextSnapshotAdapter, InMemoryHandoffAdapter,
    },
    domain::{
        AgentSession, AgentSessionState, ConversationId, HandoffStatus, SequenceNumber, TurnId,
    },
    ports::{
        agent_session::AgentSessionRepository, context_snapshot::ContextSnapshotPort,
        handoff::AgentHandoffPort,
    },
    services::HandoffService,
};
use mockable::DefaultClock;
use rstest::{fixture, rstest};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Provides a tokio runtime for async operations in tests.
#[fixture]
fn runtime() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to create runtime")
}

/// Provides a clock for time-dependent operations.
#[fixture]
fn clock() -> DefaultClock {
    DefaultClock
}

/// A test harness containing all components needed for handoff testing.
struct HandoffTestHarness {
    session_repo: Arc<InMemoryAgentSessionRepository>,
    handoff_adapter: Arc<InMemoryHandoffAdapter<DefaultClock>>,
    snapshot_adapter: Arc<InMemoryContextSnapshotAdapter<DefaultClock>>,
    service: HandoffService<
        InMemoryAgentSessionRepository,
        InMemoryHandoffAdapter<DefaultClock>,
        InMemoryContextSnapshotAdapter<DefaultClock>,
    >,
}

impl HandoffTestHarness {
    fn new() -> Self {
        let session_repo = Arc::new(InMemoryAgentSessionRepository::new());
        let handoff_adapter = Arc::new(InMemoryHandoffAdapter::new(DefaultClock));
        let snapshot_adapter = Arc::new(InMemoryContextSnapshotAdapter::new(DefaultClock));

        let service = HandoffService::new(
            Arc::clone(&session_repo),
            Arc::clone(&handoff_adapter),
            Arc::clone(&snapshot_adapter),
        );

        Self {
            session_repo,
            handoff_adapter,
            snapshot_adapter,
            service,
        }
    }
}

#[fixture]
fn harness() -> HandoffTestHarness {
    HandoffTestHarness::new()
}

// ============================================================================
// Session Management Tests
// ============================================================================

#[rstest]
fn create_session_from_handoff_stores_correctly(runtime: Runtime, harness: HandoffTestHarness) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();
        let handoff_id = corbusier::message::domain::HandoffId::new();

        let session = harness
            .service
            .create_target_session(
                conversation_id,
                "target-agent",
                SequenceNumber::new(10),
                handoff_id,
            )
            .await
            .expect("should create session");

        assert_eq!(session.conversation_id, conversation_id);
        assert_eq!(session.agent_backend, "target-agent");
        assert_eq!(session.initiated_by_handoff, Some(handoff_id));
        assert_eq!(session.state, AgentSessionState::Active);

        // Verify persistence
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
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // Create and store a session
        let session = AgentSession::new(conversation_id, "agent-1", SequenceNumber::new(1), &clock);

        harness
            .session_repo
            .store(&session)
            .await
            .expect("should store");

        // Find active session
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
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // Create multiple sessions
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

// ============================================================================
// Handoff Initiation Tests
// ============================================================================

#[rstest]
fn initiate_handoff_requires_active_session(runtime: Runtime, harness: HandoffTestHarness) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();
        let session_id = corbusier::message::domain::AgentSessionId::new();

        // Try to initiate handoff without an active session
        let result = harness
            .service
            .initiate(
                conversation_id,
                session_id,
                "target-agent",
                TurnId::new(),
                SequenceNumber::new(5),
                Some("task too complex"),
            )
            .await;

        assert!(result.is_err());
    });
}

#[rstest]
fn initiate_handoff_succeeds_with_active_session(
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // Create an active session
        let session = AgentSession::new(
            conversation_id,
            "source-agent",
            SequenceNumber::new(1),
            &clock,
        );

        harness.session_repo.store(&session).await.expect("store");

        // Initiate handoff
        let handoff = harness
            .service
            .initiate(
                conversation_id,
                session.session_id,
                "target-agent",
                TurnId::new(),
                SequenceNumber::new(5),
                Some("task requires specialist"),
            )
            .await
            .expect("should initiate");

        assert_eq!(handoff.source_session_id, session.session_id);
        assert_eq!(handoff.source_agent, "source-agent");
        assert_eq!(handoff.target_agent, "target-agent");
        assert_eq!(handoff.status, HandoffStatus::Initiated);
        assert_eq!(handoff.reason, Some("task requires specialist".to_owned()));

        // Verify source session updated
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

// ============================================================================
// Handoff Completion Tests
// ============================================================================

#[rstest]
fn complete_handoff_links_target_session(
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // Create source session and initiate handoff
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

        let handoff = harness
            .service
            .initiate(
                conversation_id,
                source_session.session_id,
                "target-agent",
                TurnId::new(),
                SequenceNumber::new(5),
                None,
            )
            .await
            .expect("initiate");

        // Create target session
        let target_session = harness
            .service
            .create_target_session(
                conversation_id,
                "target-agent",
                SequenceNumber::new(6),
                handoff.handoff_id,
            )
            .await
            .expect("create target");

        // Complete handoff
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

// ============================================================================
// Handoff Cancellation Tests
// ============================================================================

#[rstest]
fn cancel_handoff_reverts_source_session(
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // Create source session and initiate handoff
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

        let handoff = harness
            .service
            .initiate(
                conversation_id,
                source_session.session_id,
                "target-agent",
                TurnId::new(),
                SequenceNumber::new(5),
                None,
            )
            .await
            .expect("initiate");

        // Cancel the handoff
        harness
            .service
            .cancel(handoff.handoff_id, Some("target agent unavailable"))
            .await
            .expect("cancel");

        // Verify source session reverted
        let reverted = harness
            .session_repo
            .find_by_id(source_session.session_id)
            .await
            .expect("find")
            .expect("exists");

        assert_eq!(reverted.state, AgentSessionState::Active);
        assert_eq!(reverted.terminated_by_handoff, None);
    });
}

// ============================================================================
// Context Snapshot Tests
// ============================================================================

#[rstest]
fn handoff_captures_context_snapshot(
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // Create source session and initiate handoff
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

        let _handoff = harness
            .service
            .initiate(
                conversation_id,
                source_session.session_id,
                "target-agent",
                TurnId::new(),
                SequenceNumber::new(5),
                None,
            )
            .await
            .expect("initiate");

        // Verify snapshot was captured
        let snapshots = harness
            .snapshot_adapter
            .find_snapshots_for_session(source_session.session_id)
            .await
            .expect("find snapshots");

        assert_eq!(snapshots.len(), 1);
        assert_eq!(
            snapshots[0].snapshot_type,
            corbusier::message::domain::SnapshotType::HandoffInitiated
        );
    });
}

// ============================================================================
// Pending Handoff Query Tests
// ============================================================================

#[rstest]
fn get_pending_handoff_returns_initiated(
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // Create source session and initiate handoff
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

        let handoff = harness
            .service
            .initiate(
                conversation_id,
                source_session.session_id,
                "target-agent",
                TurnId::new(),
                SequenceNumber::new(5),
                None,
            )
            .await
            .expect("initiate");

        // Query pending handoff
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
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // Create source session and complete the handoff
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

        let handoff = harness
            .service
            .initiate(
                conversation_id,
                source_session.session_id,
                "target-agent",
                TurnId::new(),
                SequenceNumber::new(5),
                None,
            )
            .await
            .expect("initiate");

        // Create target and complete
        let target = harness
            .service
            .create_target_session(
                conversation_id,
                "target-agent",
                SequenceNumber::new(6),
                handoff.handoff_id,
            )
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

        // Should find no pending handoff
        let pending = harness
            .service
            .get_pending_handoff(conversation_id)
            .await
            .expect("query");

        assert!(pending.is_none());
    });
}

// ============================================================================
// Multiple Handoff Chain Tests
// ============================================================================

#[rstest]
fn handoff_chain_tracks_all_sessions(
    runtime: Runtime,
    harness: HandoffTestHarness,
    clock: DefaultClock,
) {
    runtime.block_on(async {
        let conversation_id = ConversationId::new();

        // First agent session
        let agent1 = AgentSession::new(conversation_id, "agent-1", SequenceNumber::new(1), &clock);
        harness.session_repo.store(&agent1).await.expect("store 1");

        // First handoff: agent-1 -> agent-2
        let handoff1 = harness
            .service
            .initiate(
                conversation_id,
                agent1.session_id,
                "agent-2",
                TurnId::new(),
                SequenceNumber::new(5),
                Some("escalate to specialist"),
            )
            .await
            .expect("initiate 1");

        let agent2 = harness
            .service
            .create_target_session(
                conversation_id,
                "agent-2",
                SequenceNumber::new(6),
                handoff1.handoff_id,
            )
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

        // Second handoff: agent-2 -> agent-3
        let handoff2 = harness
            .service
            .initiate(
                conversation_id,
                agent2.session_id,
                "agent-3",
                TurnId::new(),
                SequenceNumber::new(10),
                Some("need domain expert"),
            )
            .await
            .expect("initiate 2");

        let agent3 = harness
            .service
            .create_target_session(
                conversation_id,
                "agent-3",
                SequenceNumber::new(11),
                handoff2.handoff_id,
            )
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

        // Verify all sessions are tracked
        let sessions = harness
            .session_repo
            .find_by_conversation(conversation_id)
            .await
            .expect("list");

        assert_eq!(sessions.len(), 3);

        // Verify handoff chain
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
